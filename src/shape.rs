use std::collections::HashMap;
use std::ops::{Add, Sub};
use swf_tree as swf;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl From<swf::Vector2D> for Point {
    fn from(v: swf::Vector2D) -> Self {
        Point { x: v.x, y: v.y }
    }
}

impl Add for Point {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Point {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Point {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Point {
    pub fn x_y(self) -> (i32, i32) {
        (self.x, self.y)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub from: Point,
    pub bezier_control: Option<Point>,
    pub to: Point,
}

impl Line {
    pub fn flip_direction(self) -> Self {
        Line {
            from: self.to,
            bezier_control: self.bezier_control,
            to: self.from,
        }
    }

    pub fn map_points(self, mut f: impl FnMut(Point) -> Point) -> Self {
        Line {
            from: f(self.from),
            bezier_control: self.bezier_control.map(&mut f),
            to: f(self.to),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StyledPath<S> {
    pub style: S,
    pub path: Vec<Line>,
}

impl<S> StyledPath<S> {
    pub fn new(style: S) -> Self {
        StyledPath {
            style,
            path: vec![],
        }
    }

    // TODO(eddyb) confirm/infirm the correctness of this.
    // Suspected test case would involve self-overlapping paths.
    //
    // http://wahlers.com.br/claus/blog/hacking-swf-1-shapes-in-flash/
    // has some examples, but no SWF's to download.
    fn untangle_path(&mut self) {
        if self.path.is_empty() {
            return;
        }

        // FIXME(eddyb) optimize this with a bitset.
        let mut used: Vec<bool> = vec![false; self.path.len()];
        // TODO(eddyb) consider using a bitset instead of `Vec<usize>`.
        let mut lines_from: HashMap<Point, Vec<usize>> = HashMap::new();
        for (i, &line) in self.path.iter().enumerate() {
            lines_from.entry(line.from).or_default().push(i);
        }

        let mut new_path = Vec::with_capacity(self.path.len());

        let mut i = 0;
        loop {
            assert!(!used[i]);

            used[i] = true;
            let line = self.path[i];
            new_path.push(line);

            if new_path.len() == self.path.len() {
                break;
            }

            // Prefer continuing lines in the original order.
            let preferred_start = i;

            // Pick one of the remaining continuation lines from the map.
            let mut line_indices = lines_from
                .get(&line.to)
                .map_or(&[][..], |v| &v[..])
                .iter()
                .cloned()
                .filter(|&j| !used[j]);

            i = line_indices.next().unwrap_or_else(|| {
                // No remaining lines, start another path.
                used.iter().position(|&x| x == false).unwrap()
            });

            // FIXME(eddyb) speed this up with binary search and/or bitsets.
            if i < preferred_start {
                if let Some(j) = line_indices.find(|&j| j > preferred_start) {
                    i = j;
                }
            }
        }

        self.path = new_path;
    }
}

#[derive(Clone, Debug)]
pub struct Shape<'a> {
    pub center: Point,
    pub fill: Vec<StyledPath<&'a swf::FillStyle>>,
    pub stroke: Vec<StyledPath<&'a swf::LineStyle>>,
}

impl<'a> From<&'a swf::tags::DefineShape> for Shape<'a> {
    fn from(def: &'a swf::tags::DefineShape) -> Self {
        #[derive(Copy, Clone, Default)]
        struct Style {
            start: usize,
            current: Option<usize>,
        }

        impl Style {
            fn set_from_swf(&mut self, i: usize) {
                self.current = i.checked_sub(1).map(|i| i + self.start);
            }
        }

        #[derive(Copy, Clone, Default)]
        struct Styles {
            fill0: Style,
            fill1: Style,
            stroke: Style,
        }

        impl<'a> Shape<'a> {
            fn add_path(&mut self, path: &[Line], styles: Styles) {
                if let Some(fill0) = styles.fill0.current {
                    self.fill[fill0]
                        .path
                        .extend(path.iter().rev().map(|line| line.flip_direction()));
                }
                if let Some(fill1) = styles.fill1.current {
                    self.fill[fill1].path.extend(path);
                }
                if let Some(stroke) = styles.stroke.current {
                    self.stroke[stroke].path.extend(path);
                }
            }
        }

        let mut shape = Shape {
            center: Point {
                x: (def.bounds.x_min as i32 + def.bounds.x_max as i32) / 2,
                y: (def.bounds.y_min as i32 + def.bounds.y_max as i32) / 2,
            },
            fill: def
                .shape
                .initial_styles
                .fill
                .iter()
                .map(StyledPath::new)
                .collect(),
            stroke: def
                .shape
                .initial_styles
                .line
                .iter()
                .map(StyledPath::new)
                .collect(),
        };

        let mut pos = Point::default();
        let mut styles = Styles::default();

        let mut path = vec![];
        for record in &def.shape.records {
            match record {
                swf::ShapeRecord::StyleChange(change) => {
                    match change {
                        // Moving without changing styles stays within a path.
                        swf::shape_records::StyleChange {
                            left_fill: None,
                            right_fill: None,
                            line_style: None,
                            ..
                        } => {}

                        // If we do have a style change, switch paths.
                        _ => {
                            shape.add_path(&path, styles);
                            path.clear();
                        }
                    }

                    // Process new style definitions first, so that
                    // style updates can refer to the new styles.
                    if let Some(new_styles) = &change.new_styles {
                        styles.fill0.start = shape.fill.len();
                        styles.fill1.start = shape.fill.len();
                        shape
                            .fill
                            .extend(new_styles.fill.iter().map(StyledPath::new));
                        styles.stroke.start = shape.stroke.len();
                        shape
                            .stroke
                            .extend(new_styles.line.iter().map(StyledPath::new));
                    }

                    if let Some(move_to) = change.move_to.map(Point::from) {
                        pos = move_to;
                    }
                    if let Some(left_fill) = change.left_fill {
                        styles.fill0.set_from_swf(left_fill);
                    }
                    if let Some(right_fill) = change.right_fill {
                        styles.fill1.set_from_swf(right_fill);
                    }
                    if let Some(line_style) = change.line_style {
                        styles.stroke.set_from_swf(line_style);
                    }
                }
                swf::ShapeRecord::Edge(edge) => {
                    let line = Line {
                        from: Point::default(),
                        bezier_control: edge.control_delta.map(Point::from),
                        to: Point::from(edge.delta),
                    };
                    let line = line.map_points(|p| pos + p);
                    path.push(line);
                    pos = line.to;
                }
            };
        }

        shape.add_path(&path, styles);

        for fill in &mut shape.fill {
            fill.untangle_path();
        }
        for stroke in &mut shape.stroke {
            stroke.untangle_path();
        }

        shape
    }
}
