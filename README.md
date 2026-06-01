# foxplugs

Rust audio plugin monorepo.

## Crates

- [`foxcrush`](crates/foxcrush) — a bitcrusher CLAP/VST3 plugin.
- [`foxplugs-ui`](crates/foxplugs-ui) — shared Vizia/NIH-plug UI primitives for Foxplugs plugins.
- [`xtask`](crates/xtask) — workspace-level plugin bundling task.

## Common commands

```bash
cargo test -p foxcrush --lib --no-default-features  # fast DSP/processor tests without the GUI crate
cargo test -p foxcrush --lib                        # plugin library tests with the default GUI feature
cargo test --workspace
cargo xtask bundle foxcrush --release
```

Enable development-time audio allocation assertions with:

```bash
cargo test -p foxcrush --features dev-assert-process-allocs
```
