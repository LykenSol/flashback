# Adobe Flash / SWF preservation tools

[![Build Status](https://travis-ci.com/lykenware/flashback.svg?branch=master)](https://travis-ci.com/lykenware/flashback)
[![Latest Version](https://img.shields.io/crates/v/flashback.svg)](https://crates.io/crates/flashback)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/flashback)

The goal is to convert SWF files to more durable technologies (SVG, WASM, etc.),
avoiding on-the-fly emulation as much as possible.

## Status

*This is an **experimental** project, with no guarantees that it will ever become useful.*

### Conversion to animated SVGs

Only shapes with basic fill/stroke styles work, other "characters" are ignored.
Objects' presence and their transformations are converted to SVG `<animate>`
and `<animateTransform>` applied to groups of paths.

It's possible to support more SWF features (e.g. sprites), but there are
fundamental limitations, at least for script-less SVG.

`cargo run foo.swf` will output a `foo.svg` file, which hopefully resembles
the original, at least partially. It can also process multiple files, so you
can use `cargo run your-flash-stash/*.swf` to get a representative sample.

Feel free to experiment and report issues, but keep in mind that errors
explicitly mentioning `swf-parser` or `Unknown(Unknown {...})` tags are
caused by limitations in the [Open Flash] components (see below).

## Relation to the [Open Flash] project

[Open Flash]'s goals align well with this project, and its components will be
used where they suffice, to avoid wasting time reinventing the wheel.

It's possible that contributions will be made to [Open Flash] components if
necessary, and this project might even be merged into [Open Flash]
(if it gets past the experimental stage).

See also their ["Related projects"](https://github.com/open-flash/open-flash#related-projects) section.

[Open Flash]: https://github.com/open-flash/open-flash#open-flash

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
