use crate::dictionary::{Character, CharacterId, Dictionary};
use crate::export::js;
use crate::shape::{Line, Shape};
use crate::timeline::{Frame, Timeline, TimelineBuilder};
use std::cell::Cell;
use std::collections::BTreeSet;
use svg::node::element::{
    path, ClipPath, Definitions, Group, LinearGradient, Path, RadialGradient, Rectangle, Script,
    Stop,
};
use swf_tree as swf;

// FIXME(eddyb) upstream these as methods on `swf-fixed` types.
fn ufixed8p8_epsilons(x: &swf::fixed_point::Ufixed8P8) -> u16 {
    unsafe { std::mem::transmute_copy(x) }
}
fn ufixed8p8_to_f64(x: &swf::fixed_point::Ufixed8P8) -> f64 {
    ufixed8p8_epsilons(x) as f64 / (1 << 8) as f64
}

mod animate;

#[derive(Default)]
pub struct Config {
    pub use_js: bool,
}

pub fn export(movie: &swf::Movie, config: Config) -> svg::Document {
    let mut dictionary = Dictionary::default();

    let mut bg = [0, 0, 0];
    let mut timeline_builder = TimelineBuilder::default();
    for tag in &movie.tags {
        match tag {
            swf::Tag::SetBackgroundColor(set_bg) => {
                let c = &set_bg.color;
                bg = [c.r, c.g, c.b];
            }
            swf::Tag::DefineShape(def) => {
                dictionary.define(CharacterId(def.id), Character::Shape(Shape::from(def)))
            }
            swf::Tag::DefineSprite(def) => {
                let mut timeline_builder = TimelineBuilder::default();
                for tag in &def.tags {
                    match tag {
                        swf::Tag::PlaceObject(place) => timeline_builder.place_object(place),
                        swf::Tag::RemoveObject(remove) => timeline_builder.remove_object(remove),
                        swf::Tag::DoAction(do_action) => timeline_builder.do_action(do_action),
                        swf::Tag::ShowFrame => timeline_builder.advance_frame(),
                        _ => eprintln!("unknown sprite tag: {:?}", tag),
                    }
                }
                let timeline = timeline_builder.finish(Frame(def.frame_count as u16));
                dictionary.define(CharacterId(def.id), Character::Sprite(timeline))
            }
            swf::Tag::DefineDynamicText(def) => {
                dictionary.define(CharacterId(def.id), Character::DynamicText(def))
            }
            swf::Tag::PlaceObject(place) => timeline_builder.place_object(place),
            swf::Tag::RemoveObject(remove) => timeline_builder.remove_object(remove),
            swf::Tag::DoAction(do_action) => timeline_builder.do_action(do_action),
            swf::Tag::ShowFrame => timeline_builder.advance_frame(),
            _ => eprintln!("unknown tag: {:?}", tag),
        }
    }
    let timeline = timeline_builder.finish(Frame(movie.header.frame_count));

    let view_box = {
        let r = &movie.header.frame_size;
        (r.x_min, r.y_min, r.x_max - r.x_min, r.y_max - r.y_min)
    };

    let cx = Context {
        config,
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
    cx.each_used_character(&timeline, &mut |c| {
        used_characters.insert(c);
    });

    let mut js_sprites = js::code! {};
    for character_id in used_characters {
        let character = match cx.dictionary.get(character_id) {
            Some(character) => character,
            None => {
                eprintln!("missing dictionary entry for {:?}", character_id);
                continue;
            }
        };
        let svg_character = cx.export_character(character);
        cx.add_svg_def(svg_character.set("id", format!("c_{}", character_id.0)));

        if cx.config.use_js {
            if let Character::Sprite(timeline) = character {
                js_sprites += js::code! {
                    "sprites[", character_id.0, "] = ", js::timeline::export(timeline), ";\n"
                };
            }
        }
    }

    let mut svg_document = svg::Document::new()
        .set("viewBox", view_box)
        .set("style", "background: black")
        .add(
            Rectangle::new()
                .set("width", "100%")
                .set("height", "100%")
                .set("fill", format!("#{:02x}{:02x}{:02x}", bg[0], bg[1], bg[2])),
        );

    if !cx.config.use_js {
        let svg_body = cx
            .export_timeline(&timeline)
            .set("clip-path", "url(#viewBox_clip)");
        svg_document = svg_document.add(cx.svg_defs.into_inner()).add(svg_body);
    } else {
        svg_document = svg_document
            .add(cx.svg_defs.into_inner())
            .add(
                Group::new()
                    .set("id", "body")
                    .set("clip-path", "url(#viewBox_clip)"),
            )
            .add(
                js::code! {
                    "var timeline = ", js::timeline::export(&timeline), ";\n",
                    "var sprites = [];\n",
                    js_sprites,
                    "var frame_rate = ", cx.frame_rate, ";\n\n",
                    include_str!("runtime.js")
                }
                .to_svg(),
            );
    }

    svg_document
}

impl js::Code {
    fn to_svg(self) -> Script {
        Script::new(
            js::code! {
                "// <![CDATA[\n",
                self, "\n",
                "// ]]>\n"
            }
            .0,
        )
    }
}

struct Context<'a> {
    config: Config,
    frame_rate: f64,
    dictionary: Dictionary<'a>,
    svg_defs: Cell<Definitions>,
    next_gradient_id: Cell<usize>,
}

impl<'a> Context<'a> {
    fn each_used_character(&self, timeline: &Timeline, f: &mut impl FnMut(CharacterId)) {
        for layer in timeline.layers.values() {
            for obj in layer.frames.values() {
                if let Some(obj) = obj {
                    f(obj.character);
                    if let Some(Character::Sprite(timeline)) = self.dictionary.get(obj.character) {
                        self.each_used_character(timeline, f);
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

    fn export_character(&self, character: &Character) -> Group {
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
            Character::Sprite(timeline) => self.export_timeline(timeline),

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

    fn export_timeline(&self, timeline: &Timeline) -> Group {
        let frame_duration = 1.0 / self.frame_rate;
        let movie_duration = timeline.frame_count.0 as f64 * frame_duration;

        let mut g = Group::new();
        if self.config.use_js {
            return g;
        }
        for layer in timeline.layers.values() {
            let mut animation = animate::ObjectAnimation::new(timeline.frame_count, movie_duration);
            for (&frame, obj) in &layer.frames {
                animation.add(frame, obj.as_ref());
            }
            g = g.add(animation.to_svg());
        }
        g
    }
}
