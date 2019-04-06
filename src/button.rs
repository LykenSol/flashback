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
                let mut mouse_ev_and_cond = [
                    (Event::HoverIn, cond.idle_to_over_up),
                    (Event::HoverOut, cond.over_up_to_idle),
                    (Event::Down, cond.over_up_to_over_down),
                    (Event::Up, cond.over_down_to_over_up),
                    (Event::DragOut, cond.over_down_to_out_down),
                    (Event::DragIn, cond.out_down_to_over_down),
                    (Event::UpOut, cond.out_down_to_idle),
                    (Event::DownIn, cond.idle_to_over_down),
                    (Event::DownOut, cond.over_down_to_idle),
                ];
                let mut key_press = cond.key_press.unwrap_or(0) as u8;

                // HACK(eddyb) The order is somewhat reversed in `swf-parser`,
                // so we have to reconstruct the flags and reparse them.
                let mut flags = key_press as u16;
                for (i, &(_, cond)) in mouse_ev_and_cond.iter().enumerate() {
                    flags |= (cond as u16) << (7 + (1 + i) % 8);
                }
                for (i, (_, cond)) in mouse_ev_and_cond.iter_mut().enumerate() {
                    *cond = (flags & (1 << i)) != 0;
                }
                key_press = (flags >> 9) as u8;

                let on = mouse_ev_and_cond
                    .iter()
                    .filter(|&&(_, cond)| cond)
                    .map(|&(ev, _)| ev)
                    .chain(Some(key_press).filter(|&k| k != 0).map(Event::KeyPress))
                    .collect();

                let actions = crate::avm1::Code::parse_and_compile(&cond_actions.actions);

                EventHandler { on, actions }
            })
            .collect();

        Button { objects, handlers }
    }
}
