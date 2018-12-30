use crate::dictionary::{Character, CharacterId, Dictionary};
use crate::scene::{Object, Scene};
use crate::shape::{Line, Point, Shape};
use svg::node::element::{path, Animate, Group, Path, Rectangle};
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
    let mut scene = Scene::default();

    let view_box = {
        let r = &movie.header.frame_size;
        (r.x_min, r.y_min, r.x_max - r.x_min, r.y_max - r.y_min)
    };
    let frame_rate = ufixed8p8_to_f64(&movie.header.frame_rate);
    let frame_duration = 1.0 / frame_rate;
    let movie_duration = movie.header.frame_count as f64 * frame_duration;
    let mut frame = 0;
    let mut svg_document = svg::Document::new().set("viewBox", view_box);

    for tag in &movie.tags {
        match tag {
            swf::Tag::SetBackgroundColor(set_bg) => {
                let bg = &set_bg.color;
                let mut bg = (bg.r, bg.g, bg.b);
                // HACK(eddyb) need black background for some reason.
                bg = (0, 0, 0);
                svg_document = svg_document.add(
                    Rectangle::new()
                        .set("width", "100%")
                        .set("height", "100%")
                        .set("fill", format!("#{:02x}{:02x}{:02x}", bg.0, bg.1, bg.2)),
                );
            }
            swf::Tag::DefineShape(def) => {
                dictionary.define(CharacterId(def.id), Character::Shape(Shape::from(def)))
            }
            swf::Tag::DefineSprite(def) => {
                dictionary.define(CharacterId(def.id), Character::Sprite(def))
            }
            swf::Tag::PlaceObject(place) => scene.place_object(place),
            swf::Tag::RemoveObject(remove) => scene.remove_object(remove),
            swf::Tag::ShowFrame => {
                let animate_set = |frame: u16, attr, val| {
                    Animate::new()
                        .set("attributeName", attr)
                        .set("to", val)
                        .set("begin", (frame as f64 * frame_duration) - movie_duration)
                        .set("dur", movie_duration)
                        .set("calcMode", "discrete")
                        .set("repeatCount", "indefinite")
                };
                svg_document = svg_document.add(
                    render_frame(&dictionary, &scene)
                        .set("opacity", 0)
                        .add(animate_set(frame, "opacity", 1))
                        .add(animate_set(frame + 1, "opacity", 0)),
                );
                frame += 1;
            }
            _ => eprintln!("unknown tag: {:?}", tag),
        }
    }

    svg_document
}

fn render_frame(dictionary: &Dictionary, scene: &Scene) -> Group {
    let mut g = Group::new();
    for obj in scene.objects_by_depth() {
        g = g.add(render_object(dictionary, obj));
    }
    g
}

fn render_object(dictionary: &Dictionary, obj: &Object) -> Group {
    let transform = {
        // TODO(eddyb) try to do this with fixed-point arithmetic,
        // and/or with SVG transforms instead of baking it.
        let sx = sfixed16p16_to_f64(&obj.matrix.scale_x);
        let sy = sfixed16p16_to_f64(&obj.matrix.scale_y);
        let rsk0 = sfixed16p16_to_f64(&obj.matrix.rotate_skew0);
        let rsk1 = sfixed16p16_to_f64(&obj.matrix.rotate_skew1);
        let translate = Point {
            x: obj.matrix.translate_x,
            y: obj.matrix.translate_y,
        };

        move |Point { x, y }| {
            let (x, y) = (x as f64, y as f64);
            let (x, y) = (x * sx + y * rsk1, x * rsk0 + y * sy);
            let (x, y) = (x.round() as i32, y.round() as i32);
            Point { x, y } + translate
        }
    };

    let mut g = Group::new();
    match dictionary.get(obj.character) {
        Some(Character::Shape(shape)) => {
            // TODO(eddyb) confirm/infirm the correctness of this.
            let transform = |p| transform(p - shape.center) + shape.center;

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
                let start = transform(path.first()?.from);

                let mut data = path::Data::new().move_to(start.x_y());
                let mut pos = start;

                for line in path.iter().map(|line| line.map_points(transform)) {
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
        Some(Character::Sprite(def)) => {
            eprintln!("unimplemented sprite: {:?}", def);
        }
        None => {
            eprintln!("missing dictionary entry for {:?}", obj.character);
        }
    }
    g
}
