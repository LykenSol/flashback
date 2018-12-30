use std::{env, fs, path::PathBuf};

fn main() {
    let path = PathBuf::from(env::args().nth(1).unwrap());
    let data = fs::read(&path).unwrap();
    let movie = swf_parser::parsers::movie::parse_movie(&data[..])
        .to_full_result()
        .unwrap();
    // println!("{:#?}", movie);
    let document = flashback::render::animated_svg::render(&movie);
    svg::save(path.with_extension("svg"), &document).unwrap();
}
