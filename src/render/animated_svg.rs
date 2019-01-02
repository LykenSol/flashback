use crate::dictionary::{Character, CharacterId, Dictionary};
use crate::scene::{Frame, SceneBuilder};
use crate::shape::{Line, Shape};
use std::collections::BTreeSet;
use std::f64::consts::PI;
use std::fmt::Write;
use svg::node::element::{
    path, Animate, AnimateTransform, Definitions, Group, Path, Rectangle, Use,
};
use swf_tree as swf;

// FIXME(eddyb) upstream these as methods on `swf-fixed` types.
fn ufixed8p8_epsilons(x: &swf::fixed_point::Ufixed8P8) -> u16 {
    unsafe { std::mem::transmute_copy(x) }
}
fn sfixed16p16_epsilons(x: &swf::fixed_point::Sfixed16P16) -> i32 {
    unsafe { std::mem::transmute_copy(x) }
}
fn ufixed8p8_to_f64(x: &swf::fixed_point::Ufixed8P8) -> f64 {
    ufixed8p8_epsilons(x) as f64 / (1 << 8) as f64
}
fn sfixed16p16_to_f64(x: &swf::fixed_point::Sfixed16P16) -> f64 {
    sfixed16p16_epsilons(x) as f64 / (1 << 16) as f64
}

pub fn render(movie: &swf::Movie) -> svg::Document {
    let mut dictionary = Dictionary::default();
    let mut scene_builder = SceneBuilder::default();

    let mut bg = [0, 0, 0];

    for tag in &movie.tags {
        match tag {
            swf::Tag::SetBackgroundColor(set_bg) => {
                let c = &set_bg.color;
                // HACK(eddyb) need black background for some reason.
                // bg = [c.r, c.g, c.b];
            }
            swf::Tag::DefineShape(def) => {
                dictionary.define(CharacterId(def.id), Character::Shape(Shape::from(def)))
            }
            swf::Tag::DefineSprite(def) => {
                dictionary.define(CharacterId(def.id), Character::Sprite(def))
            }
            swf::Tag::PlaceObject(place) => scene_builder.place_object(place),
            swf::Tag::RemoveObject(remove) => scene_builder.remove_object(remove),
            swf::Tag::ShowFrame => scene_builder.advance_frame(),
            _ => eprintln!("unknown tag: {:?}", tag),
        }
    }

    let scene = scene_builder.finish(movie);

    let view_box = {
        let r = &movie.header.frame_size;
        (r.x_min, r.y_min, r.x_max - r.x_min, r.y_max - r.y_min)
    };
    let mut svg_document = svg::Document::new().set("viewBox", view_box).add(
        Rectangle::new()
            .set("width", "100%")
            .set("height", "100%")
            .set("fill", format!("#{:02x}{:02x}{:02x}", bg[0], bg[1], bg[2])),
    );

    let mut used_characters = BTreeSet::new();
    for (&(_, character), layer) in &scene.layers {
        if layer.frames.values().any(|obj| obj.show) {
            used_characters.insert(character);
        }
    }

    let mut svg_defs = Definitions::new();
    for character in used_characters {
        let id = format!("c_{}", character.0);
        let character = match dictionary.get(character) {
            Some(character) => character,
            None => {
                eprintln!("missing dictionary entry for {:?}", character);
                continue;
            }
        };
        svg_defs = svg_defs.add(render_character(character).set("id", id));
    }

    svg_document = svg_document.add(svg_defs);

    let frame_count = Frame(movie.header.frame_count);

    let frame_rate = ufixed8p8_to_f64(&movie.header.frame_rate);
    let frame_duration = 1.0 / frame_rate;
    let movie_duration = frame_count.0 as f64 * frame_duration;

    for (&(_, character), layer) in &scene.layers {
        let mut opacity = Animation::new(frame_count, movie_duration, 1);

        let mut scale = Animation::new(frame_count, movie_duration, (1.0, 1.0));
        let mut skew_y = Animation::new(frame_count, movie_duration, 0.0);
        let mut rotate = Animation::new(frame_count, movie_duration, 0.0);
        let mut translate = Animation::new(frame_count, movie_duration, (0, 0));

        for (&frame, obj) in &layer.frames {
            opacity.add(frame, obj.show as u8);
            if !obj.show {
                continue;
            }

            let transform = Transform::from(&obj.matrix);

            scale.add(frame, transform.scale);
            skew_y.add(frame, transform.skew_y);
            rotate.add(frame, transform.rotate);
            translate.add(frame, transform.translate);
        }

        // FIXME(eddyb) try to get rid of the redundant `<g>` here.
        let mut g = Group::new().add(Use::new().set("href", format!("#c_{}", character.0)));
        g = opacity.animate(g, "opacity");

        g = scale.animate_transform(g, "scale");
        g = skew_y.animate_transform(g, "skewY");
        g = rotate.animate_transform(g, "rotate");
        g = translate.animate_transform(g, "translate");

        svg_document = svg_document.add(g);
    }

    svg_document
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

    fn animate(self, g: Group, attr: &str) -> Group {
        match &self.key_times[..] {
            "" => g,
            "0" => g.set(attr, self.values),
            _ => g.add(
                Animate::new()
                    .set("attributeName", attr)
                    .set("keyTimes", self.key_times)
                    .set("values", self.values)
                    .set("calcMode", "discrete")
                    .set("repeatCount", "indefinite")
                    .set("dur", self.movie_duration),
            ),
        }
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

// TODO(eddyb) render frames as SVG animations of each character
// at a depth level, which only gets its transform animated.
fn render_character(character: &Character) -> Group {
    let mut g = Group::new();

    match character {
        Character::Shape(shape) => {
            // TODO(eddyb) do the transforms need to take `shape.center` into account?

            let fill_color = |style: &swf::FillStyle| {
                match style {
                    swf::FillStyle::Solid(solid) => {
                        let c = &solid.color;
                        if c.a == 0xff {
                            format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
                        } else {
                            format!("rgba({}, {}, {}, {})", c.r, c.g, c.b, c.a)
                        }
                    }
                    _ => {
                        // TODO(eddyb) implement gradient & bitmap support.
                        "#ff00ff".to_string()
                    }
                }
            };

            let path_data = |path: &[Line]| {
                let start = path.first()?.from;

                let mut data = path::Data::new().move_to(start.x_y());
                let mut pos = start;

                for line in path {
                    if line.from != pos {
                        data = data.move_to(line.from.x_y());
                    }

                    if let Some(control) = line.bezier_control {
                        data =
                            data.quadratic_curve_to((control.x, control.y, line.to.x, line.to.y));
                    } else {
                        data = data.line_to(line.to.x_y());
                    }

                    pos = line.to;
                }

                Some((start, data, pos))
            };

            for fill in &shape.fill {
                if let Some((start, mut data, end)) = path_data(&fill.path) {
                    if start == end {
                        data = data.close();
                    }

                    g = g.add(
                        Path::new()
                            .set("fill", fill_color(fill.style))
                            // TODO(eddyb) confirm/infirm the correctness of this.
                            .set("fill-rule", "evenodd")
                            .set("d", data),
                    );
                }
            }

            for stroke in &shape.stroke {
                if let Some((start, mut data, end)) = path_data(&stroke.path) {
                    if !stroke.style.no_close && start == end {
                        data = data.close();
                    }

                    // TODO(eddyb) implement cap/join support.

                    g = g.add(
                        Path::new()
                            .set("fill", "none")
                            .set("stroke", fill_color(&stroke.style.fill))
                            .set("stroke-width", stroke.style.width)
                            .set("d", data),
                    );
                }
            }
        }
        Character::Sprite(def) => {
            eprintln!("unimplemented sprite: {:?}", def);
        }
    }

    g
}
