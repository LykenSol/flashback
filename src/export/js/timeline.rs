use crate::export::js;
use crate::timeline::{default_color_transform, default_matrix, Depth, Frame, Timeline};
use swf_tree as swf;

// FIXME(eddyb) upstream these as methods on `swf-fixed` types.
fn sfixed8p8_epsilons(x: &swf::fixed_point::Sfixed8P8) -> i16 {
    unsafe { std::mem::transmute_copy(x) }
}
fn sfixed16p16_epsilons(x: &swf::fixed_point::Sfixed16P16) -> i32 {
    unsafe { std::mem::transmute_copy(x) }
}
fn sfixed8p8_to_f64(x: &swf::fixed_point::Sfixed8P8) -> f64 {
    sfixed8p8_epsilons(x) as f64 / (1 << 8) as f64
}
fn sfixed16p16_to_f64(x: &swf::fixed_point::Sfixed16P16) -> f64 {
    sfixed16p16_epsilons(x) as f64 / (1 << 16) as f64
}

pub fn export_matrix(matrix: &swf::Matrix) -> js::Code {
    if *matrix == default_matrix() {
        return js::code! { "null" };
    }
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

pub fn export_color_transform(color_transform: &swf::ColorTransformWithAlpha) -> js::Code {
    if *color_transform == default_color_transform() {
        return js::code! { "null" };
    }
    js::array(
        [
            sfixed8p8_to_f64(&color_transform.red_mult),
            0.0,
            0.0,
            0.0,
            color_transform.red_add as f64 / 255.0,
            0.0,
            sfixed8p8_to_f64(&color_transform.green_mult),
            0.0,
            0.0,
            color_transform.green_add as f64 / 255.0,
            0.0,
            0.0,
            sfixed8p8_to_f64(&color_transform.blue_mult),
            0.0,
            color_transform.blue_add as f64 / 255.0,
            0.0,
            0.0,
            0.0,
            sfixed8p8_to_f64(&color_transform.alpha_mult),
            color_transform.alpha_add as f64 / 255.0,
        ]
        .iter()
        .map(|x| js::code! { x }),
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
                                    (
                                        "name",
                                        match obj.name {
                                            Some(s) => js::string(s),
                                            None => js::code! { "null" },
                                        },
                                    ),
                                    (
                                        "color_transform",
                                        export_color_transform(&obj.color_transform),
                                    ),
                                    (
                                        "ratio",
                                        match obj.ratio {
                                            Some(x) => js::code! { x },
                                            None => js::code! { "null" },
                                        },
                                    ),
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
        ("actions", {
            let last_frame = timeline
                .actions
                .keys()
                .cloned()
                .rev()
                .next()
                .unwrap_or(Frame(0));
            js::array((0..=last_frame.0).map(Frame).map(
                |frame| match timeline.actions.get(&frame) {
                    Some(codes) => js::avm1::export(codes),
                    None => js::code! {},
                },
            ))
        }),
        (
            "labels",
            js::object(
                timeline
                    .labels
                    .iter()
                    .map(|(name, frame)| (js::string(name), js::code! { frame.0 })),
            ),
        ),
        ("sounds", {
            let last_frame = timeline
                .sounds
                .keys()
                .cloned()
                .rev()
                .next()
                .unwrap_or(Frame(0));
            js::array((0..=last_frame.0).map(Frame).map(
                |frame| match timeline.sounds.get(&frame) {
                    Some(sounds) => js::array(sounds.iter().map(|id| js::code! { id.0 })),
                    None => js::code! {},
                },
            ))
        }),
        (
            "sound_stream",
            match &timeline.sound_stream {
                Some(stream) => js::object(vec![
                    ("start", js::code! { stream.start.0 }),
                    ("sound", js::sound::export_mp3(&stream.mp3)),
                ]),
                None => js::code! { "null" },
            },
        ),
        ("frame_count", js::code! { timeline.frame_count.0 }),
    ])
}
