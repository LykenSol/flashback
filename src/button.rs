use crate::dictionary::CharacterId;
use crate::timeline::{Depth, Object};
use std::collections::BTreeMap;
use swf_parser::parsers::avm1::parse_actions_string;
use swf_parser::parsers::basic_data_types::{parse_color_transform_with_alpha, parse_matrix};
use swf_tree as swf;

#[derive(Clone, Debug, Default)]
pub struct PerState<T> {
    pub up: T,
    pub over: T,
    pub down: T,
    pub hit_test: T,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Event {
    // Keyboard events.
    KeyPress(u8),

    // Mouse events.
    HoverIn,
    HoverOut,
    Down,
    Up,

    // Push button mouse events.
    DragOut,
    DragIn,
    UpOut,

    // Menu button mouse events.
    DownIn,
    DownOut,
}

#[derive(Debug)]
pub struct EventHandler {
    pub on: Vec<Event>,
    pub actions: Vec<swf::avm1::Action>,
}

pub struct Button {
    pub objects: PerState<BTreeMap<Depth, Object<'static>>>,
    pub handlers: Vec<EventHandler>,
}

pub struct DefineButton {
    pub id: CharacterId,
    pub button: Button,
}

// HACK(eddyb) move this into swf-{tree,parser}.
impl DefineButton {
    pub fn try_parse(tag: &swf::tags::Unknown) -> Option<Self> {
        if tag.code != 34 {
            return None;
        }

        let id = CharacterId(u16::from_le_bytes([tag.data[0], tag.data[1]]));
        let action_offset = u16::from_le_bytes([tag.data[3], tag.data[4]]);
        let mut data = &tag.data[5..];

        let mut objects = PerState::<BTreeMap<Depth, Object>>::default();
        while data[0] & 0xf != 0 {
            let flags = data[0];
            data = &data[1..];
            if (flags & 0x10) != 0 {
                eprintln!("unsupported button filter list");
            }
            if (flags & 0x20) != 0 {
                eprintln!("unsupported button blend mode");
            }
            if (flags & 0xf0) != 0 {
                return None;
            }

            let character = CharacterId(u16::from_le_bytes([data[0], data[1]]));
            let depth = Depth(u16::from_le_bytes([data[2], data[3]]));
            data = &data[4..];

            let (rest, matrix) = parse_matrix(data).unwrap();
            data = rest;

            let (rest, color_transform) = parse_color_transform_with_alpha(data).unwrap();
            data = rest;

            let object = Object {
                character,
                matrix,
                name: None,
                color_transform,
                ratio: None,
            };

            if (flags & 1) != 0 {
                objects.up.insert(depth, object.clone());
            }
            if (flags & 2) != 0 {
                objects.over.insert(depth, object.clone());
            }
            if (flags & 4) != 0 {
                objects.down.insert(depth, object.clone());
            }
            if (flags & 8) != 0 {
                objects.hit_test.insert(depth, object.clone());
            }
        }
        assert_eq!(data[0], 0);
        data = &data[1..];

        let mut handlers = vec![];
        while action_offset != 0 && !data.is_empty() {
            let action_size = u16::from_le_bytes([data[0], data[1]]);
            let flags = u16::from_le_bytes([data[2], data[3]]);
            data = &data[4..];

            let mut on = vec![];

            let mouse_events = &[
                Event::HoverIn,
                Event::HoverOut,
                Event::Down,
                Event::Up,
                Event::DragOut,
                Event::DragIn,
                Event::UpOut,
                Event::DownIn,
                Event::DownOut,
            ];
            for (bit, &event) in mouse_events.iter().enumerate() {
                if (flags & (1 << bit)) != 0 {
                    on.push(event);
                };
            }

            let key_code = (flags >> 9) as u8;
            if key_code != 0 {
                on.push(Event::KeyPress(key_code));
            }

            let (rest, actions) = parse_actions_string(data).unwrap();
            data = rest;

            assert_eq!(data[0], 0);
            data = &data[1..];

            handlers.push(EventHandler { on, actions });

            if action_size == 0 {
                break;
            }
        }

        Some(DefineButton {
            id,
            button: Button { objects, handlers },
        })
    }
}
