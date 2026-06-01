# foxshaper

A Foxplugs audio effect plugin scaffold for ShaperBox-style modulation tools.

The first implementation target is basic volume shaping: a tempo-style periodic
gain envelope with rate, depth, shape, phase, mix, and output gain controls.
Future phases can add custom curve editing and additional shaper lanes.

## Development

```bash
cargo test -p foxshaper --lib --no-default-features
cargo check -p foxshaper --no-default-features
cargo xtask bundle foxshaper --release
```
