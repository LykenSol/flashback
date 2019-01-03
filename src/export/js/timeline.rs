use crate::export::js;
use crate::timeline::{Depth, Frame, Timeline};
use swf_tree as swf;

// FIXME(eddyb) upstream these as methods on `swf-fixed` types.
fn sfixed16p16_epsilons(x: &swf::fixed_point::Sfixed16P16) -> i32 {
    unsafe { std::mem::transmute_copy(x) }
}
fn sfixed16p16_to_f64(x: &swf::fixed_point::Sfixed16P16) -> f64 {
    sfixed16p16_epsilons(x) as f64 / (1 << 16) as f64
}

pub fn export_matrix(matrix: &swf::Matrix) -> js::Code {
    js::array(
        [
            &matrix.scale_x,
            &matrix.rotate_skew0,
            &matrix.rotate_skew1,
            &matrix.scale_y,
        ]
        .iter()
        .map(|x| js::code! { sfixed16p16_to_f64(x) })
        .chain(
            [matrix.translate_x, matrix.translate_y]
                .iter()
                .map(|x| js::code! { x }),
        ),
    )
}

pub fn export(timeline: &Timeline) -> js::Code {
    let max_depth = timeline
        .layers
        .keys()
        .cloned()
        .rev()
        .next()
        .unwrap_or(Depth(0));
    js::object(vec![
        (
            "layers",
            js::array((0..=max_depth.0).map(Depth).map(
                |depth| match timeline.layers.get(&depth) {
                    Some(layer) => {
                        let last_frame = layer
                            .frames
                            .keys()
                            .cloned()
                            .rev()
                            .next()
                            .unwrap_or(Frame(0));
                        js::array((0..=last_frame.0).map(Frame).map(|frame| {
                            match layer.frames.get(&frame) {
                                Some(Some(obj)) => js::object(vec![
                                    ("character", js::code! { obj.character.0 }),
                                    ("matrix", export_matrix(&obj.matrix)),
                                ]),
                                Some(None) => js::code! { "null" },
                                None => js::code! {},
                            }
                        }))
                    }
                    None => js::code! {},
                },
            )),
        ),
        ("frame_count", js::code! { timeline.frame_count.0 }),
    ])
}
