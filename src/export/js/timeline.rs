use crate::export::js;
use crate::timeline::{Depth, Frame, Timeline};
use swf_tree as swf;

#[rustfmt::skip]
pub fn export_matrix(m: &swf::Matrix) -> js::Code {
    if *m == swf::Matrix::default() {
        return js::code! { "null" };
    }
    js::array(
        [
            m.scale_x, m.rotate_skew0,
            m.rotate_skew1, m.scale_y,
        ]
        .iter()
        .map(|&x| js::code! { f64::from(x) })
        .chain(
            [m.translate_x, m.translate_y]
                .iter()
                .map(|x| js::code! { x }),
        ),
    )
}

#[rustfmt::skip]
pub fn export_color_transform(c: &swf::ColorTransformWithAlpha) -> js::Code {
    if *c == swf::ColorTransformWithAlpha::default() {
        return js::code! { "null" };
    }

    js::array(
        [
            f32::from(c.red_mult) as f64, 0.0, 0.0, 0.0,
            c.red_add as f64 / 255.0,

            0.0, f32::from(c.green_mult) as f64, 0.0, 0.0,
            c.green_add as f64 / 255.0,

            0.0, 0.0, f32::from(c.blue_mult) as f64, 0.0,
            c.blue_add as f64 / 255.0,

            0.0, 0.0, 0.0, f32::from(c.alpha_mult) as f64,
            c.alpha_add as f64 / 255.0,
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
                    Some(sounds) => js::array(sounds.iter().map(|sound| {
                        js::object(vec![
                            ("character", js::code! { sound.sound_id }),
                            (
                                "no_restart",
                                js::code! { sound.sound_info.sync_no_multiple },
                            ),
                            (
                                "loops",
                                match sound.sound_info.loop_count {
                                    Some(c) => js::code! { c },
                                    None => js::code! { "null" },
                                },
                            ),
                        ])
                    })),
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
