use crate::dictionary::CharacterId;
use crate::timeline::{Depth, Object};
use std::collections::BTreeMap;
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
    pub actions: crate::avm1::Code,
}

pub struct Button {
    pub objects: PerState<BTreeMap<Depth, Object<'static>>>,
    pub handlers: Vec<EventHandler>,
}

impl<'a> From<&'a swf::tags::DefineButton> for Button {
    fn from(button: &swf::tags::DefineButton) -> Self {
        let mut objects = PerState::<BTreeMap<Depth, Object>>::default();
        for record in &button.characters {
            if !record.filters.is_empty() || record.blend_mode != swf::BlendMode::Normal {
                eprintln!("Button::from: unsupported features in {:?}", record);
            }

            let depth = Depth(record.depth);

            let object = Object {
                character: CharacterId(record.character_id),
                matrix: record.matrix,
                name: None,
                color_transform: record.color_transform.unwrap_or_default(),
                ratio: None,
            };

            if record.state_up {
                objects.up.insert(depth, object);
            }
            if record.state_over {
                objects.over.insert(depth, object);
            }
            if record.state_down {
                objects.down.insert(depth, object);
            }
            if record.state_hit_test {
                objects.hit_test.insert(depth, object);
            }
        }

        let handlers = button
            .actions
            .iter()
            .map(|cond_actions| {
                let cond = cond_actions
                    .conditions
                    .expect("ButtonCondAction missing conditions");
                let on = [
                    (Event::HoverIn, cond.idle_to_over_up),
                    (Event::HoverOut, cond.over_up_to_idle),
                    (Event::Down, cond.over_up_to_over_down),
                    (Event::Up, cond.over_down_to_over_up),
                    (Event::DragOut, cond.over_down_to_out_down),
                    (Event::DragIn, cond.out_down_to_over_down),
                    (Event::UpOut, cond.out_down_to_idle),
                    (Event::DownIn, cond.idle_to_over_down),
                    (Event::DownOut, cond.over_down_to_idle),
                ]
                .iter()
                .filter(|&&(_, cond)| cond)
                .map(|&(ev, _)| ev)
                .chain(cond.key_press.map(|key| Event::KeyPress(key as u8)))
                .collect();

                let actions = crate::avm1::Code::parse_and_compile(&cond_actions.actions);

                EventHandler { on, actions }
            })
            .collect();

        Button { objects, handlers }
    }
}
