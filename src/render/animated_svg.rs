use crate::dictionary::{Character, CharacterId, Dictionary};
use crate::scene::{Frame, Object, SceneBuilder};
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

    let frame_count = Frame(movie.header.frame_count);

    let frame_rate = ufixed8p8_to_f64(&movie.header.frame_rate);
    let frame_duration = 1.0 / frame_rate;
    let movie_duration = frame_count.0 as f64 * frame_duration;

    // HACK(eddyb) fake a `<set>` animation which
    // the `svg` crate doesn't directly support.
    let animate_set = |frame: Frame, attr, val| {
        Animate::new()
            .set("attributeName", attr)
            .set("to", val)
            .set("begin", (frame.0 as f64 * frame_duration) - movie_duration)
            .set("dur", movie_duration)
            .set("calcMode", "discrete")
            .set("repeatCount", "indefinite")
    };

    for layer in scene.layers.values() {
        let mut frame = Frame(0);
        while frame < frame_count {
            // Process multiple identical frames as one (longer) frame.
            let mut next_frame = frame;
            loop {
                next_frame = next_frame + Frame(1);
                if next_frame >= frame_count {
                    break;
                }
                if layer.any_changes_in_frame(next_frame) {
                    break;
                }
            }

            if let Some(obj) = layer.object_at_frame(frame) {
                let mut obj = render_object(&dictionary, obj);

                if (frame, next_frame) != (Frame(0), frame_count) {
                    obj = obj
                        .set("opacity", 0)
                        .add(animate_set(frame, "opacity", 1))
                        .add(animate_set(next_frame, "opacity", 0));
                }
                svg_document = svg_document.add(obj);
            }

            frame = next_frame;
        }
    }

    svg_document
}

// TODO(eddyb) render frames as SVG animations of each character
// at a depth level, which only gets its transform animated.
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
