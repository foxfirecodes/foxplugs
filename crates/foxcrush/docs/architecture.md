# foxcrush — Architecture

Foxcrush is a stereo bitcrusher audio effect plugin, inspired by Ableton Live's Redux. It is not a measurement-faithful Redux clone; it aims for the same gritty/lo-fi territory with a small, testable Rust implementation.

## Goals

- Cross-platform audio effect plugin shipping as **CLAP** and **VST3**.
- Clean separation between host integration, parameters, processor state, pure DSP math, and editor/UI code.
- Reusable UI primitives for future Foxplugs plugins.
- Fast DSP test loop without launching a DAW or compiling the GUI stack when not needed.
- Real-time-safe audio processing: no allocations, locks, syscalls, logging, panics, or dynamic dispatch in the audio hot path.

## Non-goals

- **LV2 export.** Modern target DAWs support CLAP and/or VST3; LV2 is out of scope.
- **AU export.** Logic Pro/AU support is out of scope for the initial Rust toolchain.
- **Faithful emulation of Redux.** Foxcrush is its own effect.
- **Drag-and-drop audio export to a DAW timeline.** There is no portable CLAP/VST3 mechanism for this.
- **Built-in analyzer/waveform display** in v1.
- **Pre/post anti-aliasing filter** in v1.
- **Preset management** beyond host parameter/state handling.
- **Variable channel layouts** in v1. Foxcrush advertises and processes stereo only.

## Workspace layout

```text
crates/
  foxcrush/       # plugin crate: params, processor wiring, metadata, editor layout
  foxplugs-ui/    # shared Vizia/NIH-plug widgets and theme primitives
  xtask/          # workspace-level nih-plug bundle task
```

The plan intentionally did **not** extract a `foxplugs-dsp` crate yet. Foxcrush's DSP is still plugin-specific and small; a shared DSP crate should be added when a second plugin creates concrete reuse instead of abstracting prematurely.

## Foxcrush crate modules

```text
src/
  lib.rs        # NIH-plug Plugin/CLAP/VST3 adapter and exports
  params.rs     # FoxcrushParams and reusable private parameter builders
  processor.rs  # fixed-stereo Bitcrusher processor and effect policy
  dsp.rs        # pure sample-and-hold/quantization functions and tests
  editor.rs     # Foxcrush-specific Vizia layout, behind the `gui` feature
  main.rs       # standalone host binary entrypoint
```

### Data flow

```text
Host/DAW
  │
  ▼
Plugin::process() in lib.rs
  - reads smoothed NIH-plug parameters once per sample frame
  - computes quantization scale once per frame from continuous bit depth
  - passes mutable channel samples to the processor
  │
  ▼
Bitcrusher in processor.rs
  - owns fixed stereo channel state: [BitcrushChannelState; 2]
  - applies sample-and-hold, quantization, dry/wet mix, and output gain
  - zips samples with channel state instead of indexing, so unexpected channel counts cannot panic
  │
  ▼
dsp.rs
  - pure functions for quantization scale, quantization, and sample processing
  - no NIH-plug, Vizia, allocation, or host dependencies
```

## Parameters

Parameter IDs are stable and must be preserved for host/session compatibility:

- `bit_depth`
- `downsample`
- `mix`
- `output_gain`

Current semantics:

- **Bit Depth** remains a continuous `FloatParam`. The custom knob may snap drag/scroll movement for usability, but host automation and text entry can use fractional values.
- **Downsample** remains a continuous/fractional `FloatParam`. Fractional values drive a fractional sample-and-hold phase pattern rather than being rounded to integers.
- **Output** is stored as linear gain and displayed in dB. The knob's snap step is intentionally in linear gain units.

## Real-time audio constraints

- `Plugin::process()` performs no heap allocation and uses no locks, logging, syscalls, or panicking indexing paths.
- Channel state is a fixed stereo array because Foxcrush v1 advertises only stereo.
- `processor.rs` uses iterator `zip()` to process available samples with available states, avoiding out-of-bounds access if a host ever violates the advertised layout.
- `dsp.rs` captures the first input sample immediately after reset, avoiding the previous startup/reset sample-hold silence when downsample was greater than 1.
- Quantization scale is computed once per sample frame, not once per channel.
- Development-time process allocation assertions are available through the `dev-assert-process-allocs` feature instead of being enabled unconditionally.

## UI architecture

The default `gui` feature enables the Vizia editor. The editor is optional and recreatable; DSP continues processing if the host never opens it.

Reusable UI code lives in `foxplugs-ui`:

- `ParamKnob`
- `ParamKnobOptions`
- shared dark stylesheet/theme primitives

`ParamKnobOptions` makes reusable configuration explicit: snap step, bipolar drawing mode, diameter, and drag sensitivity. Unsafe `ParamPtr` calls remain isolated inside the widget implementation.

Foxcrush-specific editor code remains in `crates/foxcrush/src/editor.rs`: title text, parameter order, and knob snap choices.

## Build and validation

```bash
cargo test -p foxcrush --lib --no-default-features  # fast DSP/processor path
cargo test -p foxcrush --lib                        # default GUI-enabled plugin library tests
cargo test --workspace
cargo xtask bundle foxcrush --release
```

The no-default-features path keeps pure DSP/processor tests available without compiling the shared GUI crate.

## Plugin formats

| Format | Status | Notes |
|---|---|---|
| **CLAP** | Primary target | Modern, permissive plugin format supported by many target DAWs. |
| **VST3** | Secondary target | Enabled through NIH-plug; GPLv3-compatible for this project. |
| LV2 | Not supported | Out of scope. |
| AU | Not supported | Out of scope. |

## License

GPLv3, consistent with the GPLv3-compatible VST3 export path used by NIH-plug.
