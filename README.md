# foxplugs

Rust audio plugin monorepo.

## Crates

- [`foxcrush`](crates/foxcrush) — a bitcrusher CLAP/VST3 plugin.
- [`foxshaper`](crates/foxshaper) — a ShaperBox-style modulation plugin scaffold, initially focused on volume shaping.
- [`foxplugs-dsp`](crates/foxplugs-dsp) — shared real-time DSP helpers for Foxplugs plugins.
- [`foxplugs-plugin`](crates/foxplugs-plugin) — shared NIH-plug metadata and parameter helpers.
- [`foxplugs-ui`](crates/foxplugs-ui) — shared Vizia/NIH-plug UI primitives for Foxplugs plugins.
- [`xtask`](crates/xtask) — workspace-level plugin bundling task.

## Common commands

```bash
cargo test -p foxcrush --lib --no-default-features   # fast DSP/processor tests without the GUI crate
cargo test -p foxshaper --lib --no-default-features  # fast Foxshaper processor tests without the GUI crate
cargo test -p foxcrush --lib                         # plugin library tests with the default GUI feature
cargo test --workspace
cargo xtask bundle foxcrush --release
cargo xtask bundle foxshaper --release
```

Enable development-time audio allocation assertions with:

```bash
cargo test -p foxcrush --features dev-assert-process-allocs
cargo test -p foxshaper --features dev-assert-process-allocs
```
