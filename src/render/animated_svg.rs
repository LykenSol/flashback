use crate::dictionary::{Character, CharacterId, Dictionary};
use crate::scene::{Frame, Object, Scene, SceneBuilder};
use crate::shape::{Line, Shape};
use std::cell::Cell;
use std::collections::BTreeSet;
use std::f64::consts::PI;
use std::fmt::Write;
use svg::node::element::{
    path, Animate, AnimateTransform, ClipPath, Definitions, Group, LinearGradient, Path,
    RadialGradient, Rectangle, Stop, Use,
};
use svg::Node;
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

    let mut bg = [0, 0, 0];
    let mut scene_builder = SceneBuilder::default();
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
                let mut scene_builder = SceneBuilder::default();
                for tag in &def.tags {
                    match tag {
                        swf::Tag::PlaceObject(place) => scene_builder.place_object(place),
                        swf::Tag::RemoveObject(remove) => scene_builder.remove_object(remove),
                        swf::Tag::ShowFrame => scene_builder.advance_frame(),
                        _ => eprintln!("unknown sprite tag: {:?}", tag),
                    }
                }
                let scene = scene_builder.finish(Frame(def.frame_count as u16));
                dictionary.define(CharacterId(def.id), Character::Sprite(scene))
            }
            swf::Tag::DefineDynamicText(def) => {
                dictionary.define(CharacterId(def.id), Character::DynamicText(def))
            }
            swf::Tag::PlaceObject(place) => scene_builder.place_object(place),
            swf::Tag::RemoveObject(remove) => scene_builder.remove_object(remove),
            swf::Tag::ShowFrame => scene_builder.advance_frame(),
            _ => eprintln!("unknown tag: {:?}", tag),
        }
    }
    let scene = scene_builder.finish(Frame(movie.header.frame_count));

    let view_box = {
        let r = &movie.header.frame_size;
        (r.x_min, r.y_min, r.x_max - r.x_min, r.y_max - r.y_min)
    };

    let mut cx = Context {
        frame_rate: ufixed8p8_to_f64(&movie.header.frame_rate),
        dictionary,
        svg_defs: Cell::new(
            Definitions::new().add(
                ClipPath::new().set("id", "viewBox_clip").add(
                    Rectangle::new()
                        .set("x", view_box.0)
                        .set("y", view_box.1)
                        .set("width", view_box.2)
                        .set("height", view_box.3),
                ),
            ),
        ),
        next_gradient_id: Cell::new(0),
    };

    let mut used_characters = BTreeSet::new();
    cx.each_used_character(&scene, &mut |c| {
        used_characters.insert(c);
    });

    for character in used_characters {
        let svg_character = match cx.dictionary.get(character) {
            Some(character) => cx.render_character(character),
            None => {
                eprintln!("missing dictionary entry for {:?}", character);
                continue;
            }
        };
        cx.add_svg_def(svg_character.set("id", format!("c_{}", character.0)));
    }

    let svg_body = cx
        .render_scene(&scene)
        .set("clip-path", "url(#viewBox_clip)");

    let bg = format!("#{:02x}{:02x}{:02x}", bg[0], bg[1], bg[2]);
    svg::Document::new()
        .set("viewBox", view_box)
        .set("style", format!("background: {}", bg))
        .add(
            Rectangle::new()
                .set("width", "100%")
                .set("height", "100%")
                .set("fill", bg),
        )
        .add(cx.svg_defs.into_inner())
        .add(svg_body)
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

struct ObjectAnimation {
    character: Animation<CharacterUseHref>,

    scale: Animation<(f64, f64)>,
    skew_y: Animation<f64>,
    rotate: Animation<f64>,
    translate: Animation<(i32, i32)>,
}

impl ObjectAnimation {
    fn new(frame_count: Frame, movie_duration: f64) -> Self {
        ObjectAnimation {
            character: Animation::new(frame_count, movie_duration, CharacterUseHref(None)),

            scale: Animation::new(frame_count, movie_duration, (1.0, 1.0)),
            skew_y: Animation::new(frame_count, movie_duration, 0.0),
            rotate: Animation::new(frame_count, movie_duration, 0.0),
            translate: Animation::new(frame_count, movie_duration, (0, 0)),
        }
    }

    fn add(&mut self, frame: Frame, obj: Option<&Object>) {
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
    }

    fn to_svg(self) -> Group {
        // FIXME(eddyb) try to get rid of the redundant `<g>` here.
        let mut g = Group::new().add(self.character.animate(Use::new(), "href"));

        g = self.scale.animate_transform(g, "scale");
        g = self.skew_y.animate_transform(g, "skewY");
        g = self.rotate.animate_transform(g, "rotate");
        g = self.translate.animate_transform(g, "translate");

        g
    }
}

struct Context<'a> {
    frame_rate: f64,
    dictionary: Dictionary<'a>,
    svg_defs: Cell<Definitions>,
    next_gradient_id: Cell<usize>,
}

impl<'a> Context<'a> {
    fn each_used_character(&self, scene: &Scene, f: &mut impl FnMut(CharacterId)) {
        for layer in scene.layers.values() {
            for obj in layer.frames.values() {
                if let Some(obj) = obj {
                    f(obj.character);
                    if let Some(Character::Sprite(scene)) = self.dictionary.get(obj.character) {
                        self.each_used_character(scene, f);
                    }
                }
            }
        }
    }

    fn add_svg_def(&self, node: impl svg::Node) {
        self.svg_defs
            .set(self.svg_defs.replace(Definitions::new()).add(node));
    }

    fn rgba_to_svg(&self, c: &swf::StraightSRgba8) -> String {
        if c.a == 0xff {
            format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
        } else {
            format!("rgba({}, {}, {}, {})", c.r, c.g, c.b, c.a)
        }
    }

    fn fill_to_svg(&self, style: &swf::FillStyle) -> String {
        match style {
            swf::FillStyle::Solid(solid) => self.rgba_to_svg(&solid.color),
            // FIXME(eddyb) don't ignore the gradient transformation matrix.
            // TODO(eddyb) cache identical gradients.
            swf::FillStyle::LinearGradient(gradient) => {
                let mut svg_gradient = LinearGradient::new();
                for stop in &gradient.gradient.colors {
                    svg_gradient = svg_gradient.add(
                        Stop::new()
                            .set(
                                "offset",
                                format!("{}%", (stop.ratio as f64 / 255.0) * 100.0),
                            )
                            .set("stop-color", self.rgba_to_svg(&stop.color)),
                    );
                }

                let id = self.next_gradient_id.get();
                self.next_gradient_id.set(id + 1);

                self.add_svg_def(svg_gradient.set("id", format!("grad_{}", id)));

                format!("url(#grad_{})", id)
            }
            swf::FillStyle::RadialGradient(gradient) => {
                // FIXME(eddyb) remove duplication between linear and radial gradients.
                let mut svg_gradient = RadialGradient::new();
                for stop in &gradient.gradient.colors {
                    svg_gradient = svg_gradient.add(
                        Stop::new()
                            .set(
                                "offset",
                                format!("{}%", (stop.ratio as f64 / 255.0) * 100.0),
                            )
                            .set("stop-color", self.rgba_to_svg(&stop.color)),
                    );
                }

                let id = self.next_gradient_id.get();
                self.next_gradient_id.set(id + 1);

                self.add_svg_def(svg_gradient.set("id", format!("grad_{}", id)));

                format!("url(#grad_{})", id)
            }
            _ => {
                eprintln!("unsupported fill: {:?}", style);
                // TODO(eddyb) implement gradient & bitmap support.
                "#ff00ff".to_string()
            }
        }
    }

    fn render_character(&self, character: &Character) -> Group {
        match character {
            Character::Shape(shape) => {
                let mut g = Group::new();

                // TODO(eddyb) do the transforms need to take `shape.center` into account?

                let path_data = |path: &[Line]| {
                    let start = path.first()?.from;

                    let mut data = path::Data::new().move_to(start.x_y());
                    let mut pos = start;

                    for line in path {
                        if line.from != pos {
                            data = data.move_to(line.from.x_y());
                        }

                        if let Some(control) = line.bezier_control {
                            data = data
                                .quadratic_curve_to((control.x, control.y, line.to.x, line.to.y));
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
                                .set("fill", self.fill_to_svg(fill.style))
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
                                .set("stroke", self.fill_to_svg(&stroke.style.fill))
                                .set("stroke-width", stroke.style.width)
                                .set("d", data),
                        );
                    }
                }

                g
            }

            // TODO(eddyb) figure out if there's anything to be done here
            // wrt synchronizing the animiation timelines of sprites.
            Character::Sprite(scene) => self.render_scene(scene),

            Character::DynamicText(def) => {
                let mut text = svg::node::element::Text::new().add(svg::node::Text::new(
                    def.text.as_ref().map_or("", |s| &s[..]),
                ));

                if let Some(size) = def.font_size {
                    text = text.set("font-size", size);
                }

                if let Some(color) = &def.color {
                    text = text.set("fill", self.rgba_to_svg(color));
                }

                Group::new().add(text)
            }
        }
    }

    fn render_scene(&self, scene: &Scene) -> Group {
        let frame_duration = 1.0 / self.frame_rate;
        let movie_duration = scene.frame_count.0 as f64 * frame_duration;

        let mut svg_scene = Group::new();
        for layer in scene.layers.values() {
            let mut animation = ObjectAnimation::new(scene.frame_count, movie_duration);
            for (&frame, obj) in &layer.frames {
                animation.add(frame, obj.as_ref());
            }
            svg_scene = svg_scene.add(animation.to_svg());
        }
        svg_scene
    }
}
