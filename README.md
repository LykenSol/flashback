# Adobe Flash / SWF preservation tools

[![Build Status](https://travis-ci.com/lykenware/flashback.svg?branch=master)](https://travis-ci.com/lykenware/flashback)
[![Latest Version](https://img.shields.io/crates/v/flashback.svg)](https://crates.io/crates/flashback)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/flashback)

The goal is to convert SWF files to more durable technologies (SVG, WASM, etc.),
avoiding on-the-fly emulation as much as possible.

## Status

*This is an **experimental** project, with no progress/completion guarantees.*

Feel free to try it out and report issues, but keep in mind that errors
explicitly mentioning `swf-parser` or `Unknown {...}` tags are caused
by limitations in the [Open Flash] components (also see relevant section).

### Conversion to SVG+JS

This is now the default mode. As much as possible (paths, bitmaps, etc.) is
statically present in the SVG, while animations and actions are driven by JS.

Note that relying on JS means `<img>` tags in HTML can't display these SVGs,
and you need embed them, either directly, or via `<embed>` or `<object>`.

`cargo run foo.swf` will output a `foo.svg` file, which hopefully resembles
the original, at least partially. It can also process multiple files, so you
can use `cargo run your-flash-stash/*.swf` to get a representative sample.

### Conversion to animated SVGs

While currently not exposed via the CLI (see `src/bin/flashback.rs`), there
is support for producing an animated SVG, which works with `<img>`.

This uses `<animate>` and `<animateTransform>`, and supports all of the
static resources (e.g. paths and bitmaps) that the SVG+JS mode does.

There doesn't seem to be an easy way to handle sprites' independent animation,
and right now multiple instances of the same sprite are always in sync.

AVM1 (AS1/AS2) actions are not supported in this mode, although it might be
possible to handle some cases which result in deterministic animations.

SVG animations also appear to have some sort of event support, could be usable.

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
