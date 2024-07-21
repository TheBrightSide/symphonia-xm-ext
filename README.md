# XM module file parser/[Symphonia](https://docs.rs/symphonia/latest/symphonia/) format extension
### This crate is still under heavy development, for now only parsing of the patterns and of the header is implemented

This crate's goal is to parse and implement [Symphonia](https://docs.rs/symphonia/latest/symphonia/)'s either [Decoder](https://docs.rs/symphonia-core/latest/x86_64-unknown-linux-gnu/symphonia_core/codecs/trait.Decoder.html) or [FormatReader](https://docs.rs/symphonia-core/latest/symphonia_core/formats/trait.FormatReader.html) traits (or both). It also doesn't depend on any libraries outside Rust.

To try out the parser, try messing around with the only test in [tests.rs](/src/tests.rs) file. In the future there'll be more actual unit tests.

The following features are implemented:
- [x] Parsing of header
- [x] Parsing of pattern data
- [x] Parsing of instrument data
- [x] Parsing of sample data
- [ ] Unit tests
- [ ] Demuxing samples
- [ ] Generating stream
