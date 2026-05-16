# foxcrush — Architecture

A bitcrusher audio effect plugin, inspired by Ableton Live's Redux.

## Goals

- Cross-platform audio effect plugin (Linux, macOS, Windows) shipping as **VST3** and **CLAP**.
- "Inspired by Redux" character: combined bit-depth reduction and sample-rate reduction, with the gritty/lo-fi sonic flavor associated with that style of effect. Not a measurement-faithful clone.
- Pretty, themeable UI with room to grow toward animations, hover effects, and custom visualizations in future plugins.
- Fast iteration loop — be able to test DSP changes without launching a DAW.
- Fully open source under **GPLv3**.

## Non-goals

- **LV2 export.** Modern DAWs on all three platforms ship CLAP and/or VST3 support; LV2 adds toolchain complexity without meaningful coverage gain for this project.
- **AU export.** Logic Pro users are not a target audience for the initial release. AU would require a separate toolchain outside the Rust ecosystem.
- **Faithful emulation of Redux.** No effort is made to match Redux's exact frequency response, anti-aliasing behavior, or parameter curves. Foxcrush is its own effect that occupies the same sonic territory.
- **Drag-and-drop of audio files out of the plugin to a DAW timeline.** This is a host-specific capability with no standard CLAP/VST3 mechanism, and the plugin has no use case for it.
- **Built-in spectrum analyzer / waveform display** in the initial version. The architecture should not preclude adding one later, but the v1 UI is knobs only.
- **Preset management** beyond what host DAWs provide via the standard plugin parameter system.

## Tech stack

| Layer | Choice | Rationale |
|---|---|---|
| Language | **Rust** | Memory safety in real-time audio code; avoids C/C++ as a hard requirement. |
| Plugin framework | **[nih-plug](https://codeberg.org/BillyDM/nih-plug)** (BillyDM fork) | The original `robbert-vdh/nih-plug` is no longer maintained; BillyDM's hard fork on Codeberg is the active line. Rust-native, exports VST3 + CLAP from one codebase, real shipping plugins as references (Diopser, Spectral Compressor). |
| GUI library | **[vizia-plug](https://github.com/vizia/vizia-plug)** (replaces `nih_plug_vizia`) | The dedicated Vizia adapter for nih-plug, maintained by the Vizia team. Declarative, CSS-style stylesheets for theming and `:hover` states, retained-mode with an animation system, proven precedent for audio plugin UIs. |
| Windowing | **[baseview](https://github.com/RustAudio/baseview)** (via nih-plug + Vizia) | Cross-platform plugin windowing; surfaces file drag-into-plugin events natively. |
| Bundling | **`cargo xtask bundle`** (nih-plug convention) | Cargo cannot produce `.vst3` / `.clap` bundle directories directly; the `xtask` script handles platform-specific bundle layout. |
| Standalone host | **`nih_export_standalone!`** macro | Builds a JACK-backed CLI binary for testing without a DAW. On Linux + PipeWire, audio is routed in via `qjackctl`, `Carla`, or `pw-link`. |

## High-level architecture

```
┌──────────────────────────────────────────────────────────────┐
│                       foxcrush crate                          │
│                                                                │
│  ┌────────────────────┐     ┌────────────────────────────┐   │
│  │   Plugin struct    │────▶│       Params struct         │   │
│  │  (Plugin trait,    │     │  (#[derive(Params)])       │   │
│  │   ClapPlugin,      │     │   bit depth, downsample,    │   │
│  │   Vst3Plugin)      │     │   dry/wet, output gain,     │   │
│  │                    │     │   pre/post filter mode      │   │
│  │  - sample_rate     │     │                              │   │
│  │  - dsp state       │     │  Smoothed where audible:    │   │
│  │  - editor handle   │     │    Linear / Logarithmic     │   │
│  └─────────┬──────────┘     └─────────────┬──────────────┘   │
│            │                                │                  │
│            │ process(buffer)                │ Arc<Params>       │
│            ▼                                │                  │
│  ┌────────────────────┐                    │                  │
│  │    DSP module      │◀───────────────────┘                  │
│  │                    │                                        │
│  │  - sample/hold     │   read smoothed values per sample      │
│  │    downsampler     │                                        │
│  │  - mid-tread bit   │                                        │
│  │    quantizer       │                                        │
│  │  - optional        │                                        │
│  │    pre/post filter │                                        │
│  │  - dry/wet mix     │                                        │
│  └────────────────────┘                                        │
│                                                                │
│  ┌────────────────────────────────────────────────────────┐   │
│  │                    Editor (Vizia)                       │   │
│  │                                                          │   │
│  │   - knob widgets bound to Params via lenses              │   │
│  │   - stylesheet for theming + hover                       │   │
│  │   - reads Params (host-owned source of truth)            │   │
│  │   - writes Params via parameter setter context           │   │
│  └────────────────────────────────────────────────────────┘   │
│                                                                │
│  ┌────────────────────────────────────────────────────────┐   │
│  │              Export macros (at lib root)                 │   │
│  │   nih_export_clap!(Foxcrush)                            │   │
│  │   nih_export_vst3!(Foxcrush)                            │   │
│  └────────────────────────────────────────────────────────┘   │
│                                                                │
│  ┌────────────────────────────────────────────────────────┐   │
│  │      Standalone bin (nih_export_standalone!)             │   │
│  │      Used for dev iteration; JACK / PipeWire I/O          │   │
│  └────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘

                                │
                                ▼
                ┌───────────────────────────────┐
                │       xtask bundler            │
                │  produces:                     │
                │    target/bundled/Foxcrush.vst3│
                │    target/bundled/Foxcrush.clap│
                └───────────────────────────────┘
```

### Separation of concerns

- **DSP** is a pure module operating on sample buffers. It has no knowledge of plugin formats, parameter system, or UI. Testable in isolation.
- **Params** is the single source of truth for user-controllable state, owned by the host (DAW). Both the DSP and the editor read from it; only the editor writes to it (via the parameter context).
- **Editor** is optional and recreatable. The host can show/hide it at any time; DSP must continue processing whether or not the editor exists.
- **Plugin struct** is the integration layer that nih-plug calls into. It owns DSP state and hands out the editor when asked.

### Real-time audio constraints

- `process()` runs on the audio thread. No allocations, no locks, no syscalls. All buffers are pre-sized in `initialize()`.
- Parameter reads inside the audio loop use the smoothed values to avoid zipper noise on knob movement.
- The editor runs on a separate thread; it never touches DSP state directly, only the shared `Arc<Params>`.

## Plugin formats

| Format | Status | Notes |
|---|---|---|
| **CLAP** | Primary target | MIT-licensed, modern, supported by Bitwig, Reaper, FL, Studio One 7+, Live 12+. |
| **VST3** | Secondary target | The nih-plug framework itself is ISC-licensed, but its VST3 export uses `vst3-sys` bindings which are GPLv3. Any VST3 plugin built with nih-plug must therefore be GPLv3-compatible — which aligns with foxcrush's chosen license. |
| LV2 | Not supported | Out of scope. |
| AU | Not supported | Out of scope. |

## Build and distribution

- `cargo xtask bundle foxcrush --release` produces `.vst3` and `.clap` bundles in `target/bundled/`.
- `cargo run --release --bin foxcrush_standalone` launches the JACK standalone for dev testing.
- Installation paths (per-user) on Linux:
  - CLAP: `~/.clap/`
  - VST3: `~/.vst3/`

## License

GPLv3, consistent with the GPLv3-licensed `vst3-sys` bindings used by nih-plug's VST3 export. The nih-plug framework itself is ISC; vizia-plug is MIT; the GPLv3 obligation propagates only through the VST3 build path.
