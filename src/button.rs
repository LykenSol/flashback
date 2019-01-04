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

#[derive(Clone, Debug, Default)]
pub struct PerOnHalf<T> {
    pub idle: T,
    pub over_up: T,
    pub over_down: T,
    pub out_down: T,
}

#[derive(Debug)]
pub struct EventAction {
    pub on: PerOnHalf<PerOnHalf<bool>>,
    pub on_key_press: Option<u8>,
    pub actions: Vec<swf::avm1::Action>,
}

pub struct Button {
    pub objects: PerState<BTreeMap<Depth, Object<'static>>>,
    pub event_actions: Vec<EventAction>,
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

        let mut event_actions = vec![];
        while action_offset != 0 && !data.is_empty() {
            let action_size = u16::from_le_bytes([data[0], data[1]]);
            let flags = u16::from_le_bytes([data[2], data[3]]);
            data = &data[4..];

            let mut on = PerOnHalf::<PerOnHalf<_>>::default();

            on.idle.over_up = (flags & 0x01) != 0;
            on.over_up.idle = (flags & 0x02) != 0;

            on.over_up.over_down = (flags & 0x04) != 0;
            on.over_down.over_up = (flags & 0x08) != 0;

            on.over_down.out_down = (flags & 0x10) != 0;
            on.out_down.over_down = (flags & 0x20) != 0;

            on.out_down.idle = (flags & 0x40) != 0;

            on.idle.over_down = (flags & 0x80) != 0;
            on.over_down.idle = (flags & 0x100) != 0;

            let key_press = (flags >> 9) as u8;
            let on_key_press = if key_press == 0 {
                None
            } else {
                Some(key_press)
            };

            let (rest, actions) = parse_actions_string(data).unwrap();
            data = rest;

            assert_eq!(data[0], 0);
            data = &data[1..];

            event_actions.push(EventAction {
                on,
                on_key_press,
                actions,
            });

            if action_size == 0 {
                break;
            }
        }
        for ev in &event_actions {
            eprintln!("{:?}", ev);
            crate::avm1::Code::compile(&ev.actions);
        }

        Some(DefineButton {
            id,
            button: Button {
                objects,
                event_actions,
            },
        })
    }
}
