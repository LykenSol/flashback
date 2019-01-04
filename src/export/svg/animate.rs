use crate::dictionary::CharacterId;
use crate::timeline::{Frame, Object};
use std::f64::consts::PI;
use std::fmt::Write;
use svg::node::element::{Animate, AnimateTransform, Element, Filter, Group, Use};
use svg::Node;
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

struct Animation<T> {
    frame_count: Frame,
    movie_duration: f64,

    key_times: String,
    values: String,
    current_value: T,
}

impl<T: Copy + PartialEq + Into<svg::node::Value>> Animation<T> {
    fn new(frame_count: Frame, movie_duration: f64, initial_value: T) -> Self {
        Animation {
            frame_count,
            movie_duration,
            key_times: String::new(),
            values: String::new(),
            current_value: initial_value,
        }
    }

    fn add(&mut self, frame: Frame, value: T) {
        if self.current_value == value {
            return;
        }
        if frame != Frame(0) && self.key_times.is_empty() {
            self.add_without_checking(Frame(0), self.current_value);
        }
        self.add_without_checking(frame, value);
    }

    fn add_without_checking(&mut self, frame: Frame, value: T) {
        let t = frame.0 as f64 / self.frame_count.0 as f64;
        if !self.key_times.is_empty() {
            self.key_times.push(';');
            self.values.push(';');
        }
        let _ = write!(self.key_times, "{}", t);
        let _ = write!(self.values, "{}", Into::<svg::node::Value>::into(value));
        self.current_value = value;
    }

    fn animate<U: Node>(self, mut node: U, attr: &str) -> U {
        match &self.key_times[..] {
            "" => {}
            "0" => node.assign(attr, self.values),
            _ => node.append(
                Animate::new()
                    .set("attributeName", attr)
                    .set("keyTimes", self.key_times)
                    .set("values", self.values)
                    .set("calcMode", "discrete")
                    .set("repeatCount", "indefinite")
                    .set("dur", self.movie_duration),
            ),
        }
        node
    }

    fn animate_transform(self, g: Group, ty: &str) -> Group {
        match &self.key_times[..] {
            "" => g,
            // HACK(eddyb) perhaps there's a way to avoid having
            // one `<g>` nesting per transform, but right now it's
            // the only way I can compe up with to compose them.
            //
            // NB: if the transforms were grouped then they could use
            // one "transform" attribute for everything instead.
            "0" => Group::new()
                .add(g)
                .set("transform", format!("{}({})", ty, self.values)),
            _ => Group::new().add(g).add(
                AnimateTransform::new()
                    .set("attributeName", "transform")
                    .set("type", ty)
                    .set("keyTimes", self.key_times)
                    .set("values", self.values)
                    .set("calcMode", "discrete")
                    .set("repeatCount", "indefinite")
                    .set("dur", self.movie_duration),
            ),
        }
    }
}

#[derive(Copy, Clone)]
struct Transform {
    scale: (f64, f64),
    skew_y: f64,
    rotate: f64,
    translate: (i32, i32),
}

impl<'a> From<&'a swf::Matrix> for Transform {
    fn from(matrix: &swf::Matrix) -> Self {
        let a = sfixed16p16_to_f64(&matrix.scale_x);
        let b = sfixed16p16_to_f64(&matrix.rotate_skew0);
        let c = sfixed16p16_to_f64(&matrix.rotate_skew1);
        let d = sfixed16p16_to_f64(&matrix.scale_y);

        let rotate = b.atan2(a);
        let skew_y = d.atan2(c) - PI / 2.0 - rotate;

        let sx = (a * a + b * b).sqrt();
        let sy = (c * c + d * d).sqrt() * skew_y.cos();

        Transform {
            scale: (sx, sy),
            skew_y: skew_y * 180.0 / PI,
            rotate: rotate * 180.0 / PI,
            translate: (matrix.translate_x, matrix.translate_y),
        }
    }
}

impl Into<svg::node::Value> for Transform {
    fn into(self) -> svg::node::Value {
        let (tx, ty) = self.translate;
        let (sx, sy) = self.scale;
        format!(
            "translate({} {}) rotate({}) skewY({}) scale({} {})",
            tx, ty, self.rotate, self.skew_y, sx, sy,
        )
        .into()
    }
}

// TODO(eddyb) consider using linear <feComponentTransfer>.
#[derive(Copy, Clone, PartialEq)]
struct ColorMatrix {
    mul: [f64; 4],
    add: [i16; 4],
}

impl<'a> From<&'a swf::ColorTransformWithAlpha> for ColorMatrix {
    fn from(color_transform: &swf::ColorTransformWithAlpha) -> Self {
        ColorMatrix {
            mul: [
                sfixed8p8_to_f64(&color_transform.red_mult),
                sfixed8p8_to_f64(&color_transform.green_mult),
                sfixed8p8_to_f64(&color_transform.blue_mult),
                sfixed8p8_to_f64(&color_transform.alpha_mult),
            ],
            add: [
                color_transform.red_add,
                color_transform.green_add,
                color_transform.blue_add,
                color_transform.alpha_add,
            ],
        }
    }
}

impl Into<svg::node::Value> for ColorMatrix {
    fn into(self) -> svg::node::Value {
        format!(
            concat!(
                "{r} 0 0 0 {r_add} ",
                "0 {g} 0 0 {g_add} ",
                "0 0 {b} 0 {b_add} ",
                "0 0 0 {a} {a_add}",
            ),
            r = self.mul[0],
            g = self.mul[1],
            b = self.mul[2],
            a = self.mul[3],
            r_add = self.add[0] as f64 / 255.0,
            g_add = self.add[1] as f64 / 255.0,
            b_add = self.add[2] as f64 / 255.0,
            a_add = self.add[3] as f64 / 255.0,
        )
        .into()
    }
}

#[derive(Copy, Clone, PartialEq)]
struct CharacterUseHref(Option<CharacterId>);

impl Into<svg::node::Value> for CharacterUseHref {
    fn into(self) -> svg::node::Value {
        match self.0 {
            Some(id) => format!("#c_{}", id.0).into(),
            None => "#".into(),
        }
    }
}

pub struct ObjectAnimation {
    id_prefix: String,

    character: Animation<CharacterUseHref>,

    scale: Animation<(f64, f64)>,
    skew_y: Animation<f64>,
    rotate: Animation<f64>,
    translate: Animation<(i32, i32)>,

    color_matrix: Animation<ColorMatrix>,
}

impl ObjectAnimation {
    pub fn new(id_prefix: String, frame_count: Frame, movie_duration: f64) -> Self {
        ObjectAnimation {
            id_prefix,

            character: Animation::new(frame_count, movie_duration, CharacterUseHref(None)),

            scale: Animation::new(frame_count, movie_duration, (1.0, 1.0)),
            skew_y: Animation::new(frame_count, movie_duration, 0.0),
            rotate: Animation::new(frame_count, movie_duration, 0.0),
            translate: Animation::new(frame_count, movie_duration, (0, 0)),

            color_matrix: Animation::new(
                frame_count,
                movie_duration,
                ColorMatrix {
                    mul: [1.0; 4],
                    add: [0; 4],
                },
            ),
        }
    }

    pub fn add(&mut self, frame: Frame, obj: Option<&Object>) {
        let obj = match obj {
            None => {
                self.character.add(frame, CharacterUseHref(None));
                return;
            }
            Some(obj) => obj,
        };
        self.character
            .add(frame, CharacterUseHref(Some(obj.character)));

        let transform = Transform::from(&obj.matrix);

        self.scale.add(frame, transform.scale);
        self.skew_y.add(frame, transform.skew_y);
        self.rotate.add(frame, transform.rotate);
        self.translate.add(frame, transform.translate);

        self.color_matrix
            .add(frame, ColorMatrix::from(&obj.color_transform));
    }

    pub fn to_svg(self) -> Group {
        // FIXME(eddyb) try to get rid of the redundant `<g>` here.
        let mut g = Group::new();

        let mut obj = self.character.animate(Use::new(), "href");

        if !self.color_matrix.key_times.is_empty() {
            let filter_id = format!("{}filter", self.id_prefix);
            obj = obj.set("filter", format!("url(#{})", filter_id));

            g = g.add(
                Filter::new()
                    .set("id", filter_id)
                    .set("x", 0)
                    .set("y", 0)
                    .set("width", 1)
                    .set("height", 1)
                    .add(
                        self.color_matrix
                            .animate(Element::new("feColorMatrix"), "values"),
                    ),
            );
        }

        g = g.add(obj);

        g = self.scale.animate_transform(g, "scale");
        g = self.skew_y.animate_transform(g, "skewY");
        g = self.rotate.animate_transform(g, "rotate");
        g = self.translate.animate_transform(g, "translate");

        g
    }
}
