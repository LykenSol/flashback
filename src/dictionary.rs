use crate::shape::Shape;
use crate::timeline::Timeline;
use image::DynamicImage;
use std::collections::BTreeMap;
use swf_tree as swf;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharacterId(pub u16);

pub enum Character<'a> {
    Shape(Shape<'a>),
    Bitmap(DynamicImage),
    Sprite(Timeline<'a>),
    DynamicText(&'a swf::tags::DefineDynamicText),
}

#[derive(Default)]
pub struct Dictionary<'a> {
    pub characters: BTreeMap<CharacterId, Character<'a>>,
}

impl<'a> Dictionary<'a> {
    pub fn define(&mut self, id: CharacterId, character: Character<'a>) {
        assert!(
            self.characters.insert(id, character).is_none(),
            "Dictionary::define: ID {} is already taken"
        );
    }
}
