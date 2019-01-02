use crate::shape::Shape;
use crate::timeline::Timeline;
use std::collections::HashMap;
use std::ops::Index;
use swf_tree as swf;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharacterId(pub u16);

#[derive(Debug)]
pub enum Character<'a> {
    Shape(Shape<'a>),
    Sprite(Timeline),
    DynamicText(&'a swf::tags::DefineDynamicText),
}

#[derive(Default, Debug)]
pub struct Dictionary<'a> {
    characters: HashMap<CharacterId, Character<'a>>,
}

impl<'a> Dictionary<'a> {
    pub fn define(&mut self, id: CharacterId, character: Character<'a>) {
        assert!(
            self.characters.insert(id, character).is_none(),
            "Dictionary::define: ID {} is already taken"
        );
    }

    pub fn get(&self, id: CharacterId) -> Option<&Character<'a>> {
        self.characters.get(&id)
    }
}

impl<'a> Index<CharacterId> for Dictionary<'a> {
    type Output = Character<'a>;
    fn index(&self, id: CharacterId) -> &Self::Output {
        &self.characters[&id]
    }
}
