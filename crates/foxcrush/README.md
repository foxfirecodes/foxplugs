# foxcrush

A bitcrusher CLAP/VST3 audio effect plugin inspired by Ableton Live's Redux.

## Parameters

- **Bit Depth** is intentionally continuous for now. The Foxplugs knob snaps drag/scroll movement for usability, but host automation and text entry can use fractional values.
- **Downsample** is intentionally continuous/fractional for now. Fractional factors use the DSP's fractional sample-and-hold phase behavior.
- **Mix** is dry/wet blend.
- **Output** is stored as linear gain and displayed in dB; the custom knob snap step is in linear gain units.

## Development

```bash
cargo test -p foxcrush --lib --no-default-features
cargo test -p foxcrush --lib
cargo xtask bundle foxcrush --release
```

The default `gui` feature enables the Vizia editor through the shared `foxplugs-ui` crate. The no-default-features test path keeps DSP/processor tests available without compiling the GUI crate.

Development-only process-allocation assertions are available with the `dev-assert-process-allocs` feature.
