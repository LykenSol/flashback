use crate::dictionary::CharacterId;
use std::collections::BTreeMap;
use std::ops::Add;
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

// FIXME(eddyb) upstream these as methods on `swf-fixed` types.
fn sfixed16p16_epsilons(x: &swf::fixed_point::Sfixed16P16) -> i32 {
    unsafe { std::mem::transmute_copy(x) }
}

// FIXME(eddyb) upstream these as `#[derive(Copy, Clone)]`.
fn copy_sfixed16p16(x: &swf::fixed_point::Sfixed16P16) -> swf::fixed_point::Sfixed16P16 {
    swf::fixed_point::Sfixed16P16::from_epsilons(sfixed16p16_epsilons(x))
}

fn copy_matrix(matrix: &swf::Matrix) -> swf::Matrix {
    swf::Matrix {
        scale_x: copy_sfixed16p16(&matrix.scale_x),
        scale_y: copy_sfixed16p16(&matrix.scale_y),
        rotate_skew0: copy_sfixed16p16(&matrix.rotate_skew0),
        rotate_skew1: copy_sfixed16p16(&matrix.rotate_skew1),
        translate_x: matrix.translate_x,
        translate_y: matrix.translate_y,
    }
}

fn default_matrix() -> swf::Matrix {
    swf::Matrix {
        scale_x: swf::fixed_point::Sfixed16P16::from_epsilons(1 << 16),
        scale_y: swf::fixed_point::Sfixed16P16::from_epsilons(1 << 16),
        rotate_skew0: swf::fixed_point::Sfixed16P16::from_epsilons(0),
        rotate_skew1: swf::fixed_point::Sfixed16P16::from_epsilons(0),
        translate_x: 0,
        translate_y: 0,
    }
}

pub struct Object {
    pub show: bool,
    pub matrix: swf::Matrix,
}

impl Default for Object {
    fn default() -> Self {
        Object {
            show: false,
            matrix: default_matrix(),
        }
    }
}

impl Clone for Object {
    fn clone(&self) -> Self {
        Object {
            show: self.show,
            matrix: copy_matrix(&self.matrix),
        }
    }
}

pub struct Layer {
    pub frames: BTreeMap<Frame, Object>,
}

impl Default for Layer {
    fn default() -> Self {
        let mut frames = BTreeMap::new();
        frames.insert(Frame(0), Object::default());
        Layer { frames }
    }
}

#[derive(Default)]
pub struct Scene {
    pub layers: BTreeMap<(Depth, CharacterId), Layer>,
}

#[derive(Default)]
pub struct SceneBuilder {
    scene: Scene,
    active_characters: BTreeMap<Depth, CharacterId>,
    current_frame: Frame,
}

impl SceneBuilder {
    pub fn place_object(&mut self, place: &swf::tags::PlaceObject) {
        let depth = Depth(place.depth);

        let active_character = self.active_characters.entry(depth).or_insert_with(|| {
            place
                .character_id
                .map(CharacterId)
                .expect("SceneBuilder::place_object: missing `character_id`")
        });
        if let Some(character) = place.character_id.map(CharacterId) {
            if place.is_move {
                self.scene
                    .layers
                    .get_mut(&(depth, *active_character))
                    .unwrap()
                    .frames
                    .insert(self.current_frame, Object::default());
                *active_character = character;
            } else {
                assert_eq!(*active_character, character);
            }
        }

        let layer = self
            .scene
            .layers
            .entry((depth, *active_character))
            .or_default();

        // Find the last changed frame for this object, if it's not
        // the current one, and copy its state of the object.
        let prev_obj = match layer.frames.range(..=self.current_frame).rev().next() {
            Some((&frame, obj)) if frame != self.current_frame => obj.clone(),
            _ => Object::default(),
        };

        let obj = layer.frames.entry(self.current_frame).or_insert(prev_obj);

        if place.is_move && place.character_id.is_some() {
            *obj = Object::default();
        }
        obj.show = true;
        if let Some(matrix) = &place.matrix {
            obj.matrix = copy_matrix(matrix);
        }
    }

    pub fn remove_object(&mut self, remove: &swf::tags::RemoveObject) {
        let depth = Depth(remove.depth);

        let active_character = self
            .active_characters
            .remove(&depth)
            .expect("SceneBuilder::remove_object: no object at depth level");

        if let Some(character) = remove.character_id.map(CharacterId) {
            assert_eq!(active_character, character);
        }

        self.scene
            .layers
            .get_mut(&(depth, active_character))
            .unwrap()
            .frames
            .insert(self.current_frame, Object::default());
    }

    pub fn advance_frame(&mut self) {
        self.current_frame = self.current_frame + Frame(1);
    }

    pub fn finish(self, movie: &swf::Movie) -> Scene {
        assert_eq!(self.current_frame, Frame(movie.header.frame_count));

        self.scene
    }
}
