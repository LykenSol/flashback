use std::{env, fs, path::PathBuf};

fn main() {
    if env::args().count() == 1 {
        eprintln!("USAGE: flashback a.swf b.swf c.swf ...");
    }
    for arg in env::args().skip(1) {
        let path = PathBuf::from(arg);
        let data = fs::read(&path).unwrap();
        eprint!("{}:", path.display());
        match swf_parser::parsers::movie::parse_movie(&data[..]) {
            Ok((remaining, movie)) => {
                if !remaining.is_empty() {
                    eprintln!(
                        "swf-parser parsing incomplete: {} bytes left",
                        remaining.len()
                    );
                } else {
                    eprintln!("");
                }
                // println!("{:#?}", movie);
                let document = flashback::export::svg::export(
                    &movie,
                    flashback::export::svg::Config {
                        // TODO(eddyb) add a way to control this.
                        use_js: true,
                    },
                );
                svg::save(path.with_extension("svg"), &document).unwrap();
            }
            Err(e) => {
                eprintln!("swf-parser errored: {:?}", e);
            }
        }
    }
}
