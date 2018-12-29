# Adobe Flash / SWF preservation tools

[![Build Status](https://travis-ci.com/lykenware/flashback.svg?branch=master)](https://travis-ci.com/lykenware/flashback)
[![Latest Version](https://img.shields.io/crates/v/flashback.svg)](https://crates.io/crates/flashback)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/flashback)

The goal is to convert SWF files to more durable technologies (SVG, WASM, etc.),
avoiding on-the-fly emulation as much as possible.

## Status

This is an **experimental** project, with no guarantees that it will ever become useful.

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
