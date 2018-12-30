use crate::dictionary::CharacterId;
use std::collections::BTreeMap;
use swf_tree as swf;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Depth(pub u16);

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
    pub character: CharacterId,
    pub matrix: swf::Matrix,
}

impl Object {
    pub fn new(character: CharacterId) -> Self {
        Object {
            character,
            matrix: default_matrix(),
        }
    }
}

#[derive(Default)]
pub struct Scene {
    objects: BTreeMap<Depth, Object>,
}

impl Scene {
    pub fn objects_by_depth<'a>(&'a self) -> impl Iterator<Item = &'a Object> {
        self.objects.values()
    }

    pub fn place_object(&mut self, place: &swf::tags::PlaceObject) {
        let obj = self.objects.entry(Depth(place.depth)).or_insert_with(|| {
            Object::new(
                place
                    .character_id
                    .map(CharacterId)
                    .expect("Scene::place_object: missing `character_id`"),
            )
        });
        if let Some(character) = place.character_id.map(CharacterId) {
            if place.is_move {
                *obj = Object::new(character);
            } else {
                assert_eq!(obj.character, character);
            }
        }
        if let Some(matrix) = &place.matrix {
            obj.matrix = copy_matrix(matrix);
        }
    }

    pub fn remove_object(&mut self, remove: &swf::tags::RemoveObject) {
        let obj = self
            .objects
            .remove(&Depth(remove.depth))
            .expect("Scene::remove_object: no object at depth level");
        if let Some(character) = remove.character_id.map(CharacterId) {
            assert_eq!(obj.character, character);
        }
    }
}
