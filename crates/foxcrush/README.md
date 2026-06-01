# foxcrush

A bitcrusher CLAP/VST3 audio effect plugin inspired by Ableton Live's Redux.

## Parameters

- **Bit Depth** is a continuous control. The Foxplugs knob snaps drag/scroll movement for usability, while host automation and text entry can use fractional values.
  - Bit-depth reduction quantizes amplitude to fewer levels. Low bit depths are intentionally aggressive, but the DSP avoids low-bit zero-deadband silence by forcing nonzero samples onto a quantized level.
  - The no-zero quantizer is followed by smoothed level compensation so very low bit depths keep their crushed character without large sustained loudness boosts.
  - Higher bit depths, especially above about 6–8 bits, can sound subtle because the quantization steps become very small.
- **Downsample** is a continuous/fractional control. Fractional factors use the DSP's fractional sample-and-hold phase behavior.
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
