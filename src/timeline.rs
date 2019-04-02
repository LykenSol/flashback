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

fn copy_matrix(matrix: &swf::Matrix) -> swf::Matrix {
    swf::Matrix {
        scale_x: matrix.scale_x,
        scale_y: matrix.scale_y,
        rotate_skew0: matrix.rotate_skew0,
        rotate_skew1: matrix.rotate_skew1,
        translate_x: matrix.translate_x,
        translate_y: matrix.translate_y,
    }
}

pub fn default_matrix() -> swf::Matrix {
    swf::Matrix {
        scale_x: swf::fixed::Sfixed16P16::from_epsilons(1 << 16),
        scale_y: swf::fixed::Sfixed16P16::from_epsilons(1 << 16),
        rotate_skew0: swf::fixed::Sfixed16P16::from_epsilons(0),
        rotate_skew1: swf::fixed::Sfixed16P16::from_epsilons(0),
        translate_x: 0,
        translate_y: 0,
    }
}

fn copy_color_transform(
    color_transform: &swf::ColorTransformWithAlpha,
) -> swf::ColorTransformWithAlpha {
    swf::ColorTransformWithAlpha {
        red_mult: color_transform.red_mult,
        green_mult: color_transform.green_mult,
        blue_mult: color_transform.blue_mult,
        alpha_mult: color_transform.alpha_mult,
        red_add: color_transform.red_add,
        green_add: color_transform.green_add,
        blue_add: color_transform.blue_add,
        alpha_add: color_transform.alpha_add,
    }
}

pub fn default_color_transform() -> swf::ColorTransformWithAlpha {
    swf::ColorTransformWithAlpha {
        red_mult: swf::fixed::Sfixed8P8::from_epsilons(1 << 8),
        green_mult: swf::fixed::Sfixed8P8::from_epsilons(1 << 8),
        blue_mult: swf::fixed::Sfixed8P8::from_epsilons(1 << 8),
        alpha_mult: swf::fixed::Sfixed8P8::from_epsilons(1 << 8),
        red_add: 0,
        green_add: 0,
        blue_add: 0,
        alpha_add: 0,
    }
}

#[derive(Debug)]
pub struct Object<'a> {
    pub character: CharacterId,
    pub matrix: swf::Matrix,
    pub name: Option<&'a str>,
    pub color_transform: swf::ColorTransformWithAlpha,
    pub ratio: Option<u16>,
}

impl<'a> Clone for Object<'a> {
    fn clone(&self) -> Self {
        Object {
            character: self.character,
            matrix: copy_matrix(&self.matrix),
            name: self.name,
            color_transform: copy_color_transform(&self.color_transform),
            ratio: self.ratio,
        }
    }
}

impl<'a> Object<'a> {
    pub fn new(character: CharacterId) -> Self {
        Object {
            character,
            matrix: default_matrix(),
            name: None,
            color_transform: default_color_transform(),
            ratio: None,
        }
    }
}

#[derive(Default, Debug)]
pub struct Layer<'a> {
    pub frames: BTreeMap<Frame, Option<Object<'a>>>,
}

#[derive(Debug)]
pub struct FrameLabel<'a> {
    pub name: &'a str,
    pub anchor: bool,
}

// HACK(eddyb) move this into swf-{tree,parser}.
impl<'a> FrameLabel<'a> {
    pub fn try_parse(tag: &'a swf::tags::Unknown) -> Option<Self> {
        if tag.code != 43 {
            return None;
        }

        let mut anchor = false;
        let mut nil_pos = tag.data.len() - 1;
        if tag.data[nil_pos] != 0 {
            nil_pos -= 1;
            anchor = true;
        }
        assert_eq!(tag.data[nil_pos], 0);

        Some(FrameLabel {
            name: str::from_utf8(&tag.data[..nil_pos]).unwrap(),
            anchor,
        })
    }
}

#[derive(Debug)]
pub struct SoundStream {
    pub start: Frame,
    // FIXME(eddyb) support multiple formats.
    pub mp3: Vec<u8>,
}

#[derive(Default, Debug)]
pub struct Timeline<'a> {
    pub layers: BTreeMap<Depth, Layer<'a>>,
    pub actions: BTreeMap<Frame, Vec<avm1::Code>>,
    pub labels: BTreeMap<&'a str, Frame>,
    pub sounds: BTreeMap<Frame, Vec<sound::StartSound>>,
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
            Some((&frame, obj)) if frame != self.current_frame => obj.clone(),
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
        if let Some(matrix) = &place.matrix {
            obj.matrix = copy_matrix(matrix);
        }
        if let Some(name) = &place.name {
            obj.name = Some(name);
        }
        if let Some(color_transform) = &place.color_transform {
            obj.color_transform = copy_color_transform(color_transform);
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
            eprintln!("unsupported features in {:?}", place);
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

    pub fn frame_label(&mut self, label: FrameLabel<'a>) {
        self.timeline.labels.insert(label.name, self.current_frame);
    }

    pub fn start_sound(&mut self, sound: sound::StartSound) {
        self.timeline
            .sounds
            .entry(self.current_frame)
            .or_default()
            .push(sound);
    }

    pub fn sound_stream_head(&mut self, _head: sound::SoundStreamHead) {
        assert!(self.timeline.sound_stream.is_none());
        self.timeline.sound_stream = Some(SoundStream {
            start: self.current_frame,
            mp3: vec![],
        });
    }

    pub fn sound_stream_block(&mut self, block: sound::SoundStreamBlock) {
        match &mut self.timeline.sound_stream {
            Some(stream) => stream.mp3.extend(block.as_mp3().mp3.data),
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
