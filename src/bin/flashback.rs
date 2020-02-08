use std::{fs, path::PathBuf};
use structopt::StructOpt;

#[derive(Debug,StructOpt)]
struct Opt{
    #[structopt(long)]
    use_js:bool,
    #[structopt(required = true)]
    files: Vec<PathBuf>,
}

fn main() {
    let opt = Opt::from_args();
    for path in opt.files {
        let data = fs::read(&path).unwrap();
        eprint!("{}:", path.display());
        match swf_parser::parse_swf(&data) {
            Ok(movie) => {
                // println!("{:#?}", movie);
                let document = flashback::export::svg::export(
                    &movie,
                    flashback::export::svg::Config {
                        use_js: opt.use_js,
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
