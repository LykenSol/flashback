use crate::avm1;
use crate::dictionary::CharacterId;
use crate::sound;
use std::collections::BTreeMap;
use std::ops::Add;
use std::str;
use swf_tree as swf;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Depth(pub u16);

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Frame(pub u16);

impl Add for Frame {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Frame(self.0 + other.0)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Object<'a> {
    pub character: CharacterId,
    pub matrix: swf::Matrix,
    pub name: Option<&'a str>,
    pub color_transform: swf::ColorTransformWithAlpha,
    pub ratio: Option<u16>,
}

impl<'a> Object<'a> {
    pub fn new(character: CharacterId) -> Self {
        Object {
            character,
            matrix: swf::Matrix::default(),
            name: None,
            color_transform: swf::ColorTransformWithAlpha::default(),
            ratio: None,
        }
    }
}

#[derive(Default, Debug)]
pub struct Layer<'a> {
    pub frames: BTreeMap<Frame, Option<Object<'a>>>,
}

#[derive(Debug)]
pub struct SoundStream {
    pub start: Frame,
    pub format: swf::AudioCodingFormat,
    // FIXME(eddyb) support multiple formats.
    pub mp3: Vec<u8>,
}

#[derive(Default, Debug)]
pub struct Timeline<'a> {
    pub layers: BTreeMap<Depth, Layer<'a>>,
    pub actions: BTreeMap<Frame, Vec<avm1::Code>>,
    pub labels: BTreeMap<&'a str, Frame>,
    pub sounds: BTreeMap<Frame, Vec<&'a swf::tags::StartSound>>,
    pub sound_stream: Option<SoundStream>,
    pub frame_count: Frame,
}

#[derive(Default)]
pub struct TimelineBuilder<'a> {
    timeline: Timeline<'a>,
    current_frame: Frame,
}

impl<'a> TimelineBuilder<'a> {
    pub fn place_object(&mut self, place: &'a swf::tags::PlaceObject) {
        let layer = self.timeline.layers.entry(Depth(place.depth)).or_default();

        // Find the last changed frame for this object, if it's not
        // the current one, and copy its state of the object.
        let prev_obj = match layer.frames.range(..=self.current_frame).rev().next() {
            Some((&frame, &obj)) if frame != self.current_frame => obj,
            _ => None,
        };

        let obj = layer
            .frames
            .entry(self.current_frame)
            .or_insert(prev_obj)
            .get_or_insert_with(|| {
                Object::new(
                    place
                        .character_id
                        .map(CharacterId)
                        .expect("TimelineBuilder::place_object: missing `character_id`"),
                )
            });

        if let Some(character) = place.character_id.map(CharacterId) {
            if place.is_update {
                *obj = Object::new(character);
            } else {
                assert_eq!(obj.character, character);
            }
        }
        if let Some(matrix) = place.matrix {
            obj.matrix = matrix;
        }
        if let Some(name) = &place.name {
            obj.name = Some(name);
        }
        if let Some(color_transform) = place.color_transform {
            obj.color_transform = color_transform;
        }
        if let Some(ratio) = place.ratio {
            obj.ratio = Some(ratio);
        }

        if place.class_name.is_some()
            || place.clip_depth.is_some()
            || place.filters.is_some()
            || place.blend_mode.is_some()
            || place.visible.is_some()
            || place.background_color.is_some()
            || place.clip_actions.is_some()
        {
            eprintln!(
                "TimelineBuilder::place_object: unsupported features in {:?}",
                place
            );
        }
    }

    pub fn remove_object(&mut self, remove: &swf::tags::RemoveObject) {
        self.timeline
            .layers
            .get_mut(&Depth(remove.depth))
            .unwrap()
            .frames
            .insert(self.current_frame, None);
    }

    pub fn do_action(&mut self, do_action: &'a swf::tags::DoAction) {
        let mut data = &do_action.actions[..];
        let mut actions = vec![];
        while data[0] != 0 {
            let (rest, action) = avm1_parser::parse_action(data).unwrap();
            data = rest;
            actions.push(action);
        }
        assert_eq!(data, [0]);

        self.timeline
            .actions
            .entry(self.current_frame)
            .or_default()
            .push(avm1::Code::compile(actions))
    }

    pub fn frame_label(&mut self, label: &'a swf::tags::FrameLabel) {
        self.timeline.labels.insert(&label.name, self.current_frame);
    }

    pub fn start_sound(&mut self, sound: &'a swf::tags::StartSound) {
        if sound.sound_info.envelope_records.is_some()
            || sound.sound_info.in_point.is_some()
            || sound.sound_info.out_point.is_some()
            || sound.sound_info.sync_stop
        {
            eprintln!(
                "TimelineBuilder::start_sound: unsupported SoundInfo: {:?}",
                sound
            );
        }
        self.timeline
            .sounds
            .entry(self.current_frame)
            .or_default()
            .push(sound);
    }

    pub fn sound_stream_head(&mut self, head: &swf::tags::SoundStreamHead) {
        assert!(self.timeline.sound_stream.is_none());
        self.timeline.sound_stream = Some(SoundStream {
            start: self.current_frame,
            format: head.stream_format,
            mp3: vec![],
        });
    }

    pub fn sound_stream_block(&mut self, block: &swf::tags::SoundStreamBlock) {
        match &mut self.timeline.sound_stream {
            Some(stream) => {
                let mp3 = match stream.format {
                    swf::AudioCodingFormat::Mp3 => sound::Mp3StreamBlock::from(block).mp3,
                    _ => {
                        eprintln!(
                            "TimelineBuilder::sound_stream_block: unsupported format: {:?}",
                            stream.format,
                        );
                        return;
                    }
                };
                stream.mp3.extend(mp3.data);
            }
            None => {
                eprintln!(
                    "TimelineBuilder::sound_stream_block: unsupported {:?}",
                    block,
                );
            }
        }
    }

    pub fn advance_frame(&mut self) {
        self.current_frame = self.current_frame + Frame(1);
    }

    pub fn finish(mut self, frame_count: Frame) -> Timeline<'a> {
        // HACK(eddyb) this should be an error but it happens during testing.
        if self.current_frame != frame_count {
            eprintln!(
                "TimelineBuilder::finish: expected {} frames, found {}",
                frame_count.0, self.current_frame.0,
            );
        }
        self.timeline.frame_count = frame_count;

        self.timeline
    }
}
