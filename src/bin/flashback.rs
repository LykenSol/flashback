use std::{env, fs, path::PathBuf};

fn main() {
    if env::args().count() == 1 {
        eprintln!("USAGE: flashback a.swf b.swf c.swf ...");
    }
    for arg in env::args().skip(1) {
        let path = PathBuf::from(arg);
        let data = fs::read(&path).unwrap();
        match swf_parser::parsers::movie::parse_movie(&data[..]).to_full_result() {
            Ok(movie) => {
                eprintln!("{}:", path.display());
                // println!("{:#?}", movie);
                let document = flashback::export::svg::export(&movie);
                svg::save(path.with_extension("svg"), &document).unwrap();
            }
            Err(e) => {
                eprintln!("{}: swf-parser errored: {:?}", path.display(), e);
            }
        }
    }
}
