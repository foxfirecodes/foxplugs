# Foxcrush refactor architecture review

Scope: reviewed the current `foxplugs` workspace, `foxcrush` crate structure, docs, and build/test behavior. Requested `plan.md` and `progress.md` were not present at `/home/foxfire/code/foxplugs/` during review, so this is based on the repository state only.

## Review

- Correct: the current code already has a useful first separation between plugin integration (`crates/foxcrush/src/lib.rs`), DSP (`crates/foxcrush/src/dsp.rs`), editor (`crates/foxcrush/src/editor.rs`), and widgets (`crates/foxcrush/src/widgets/*`). `dsp::process_sample()` is independent of nih-plug/Vizia types (`crates/foxcrush/src/dsp.rs:14-28`), and `Plugin::initialize()` preallocates channel state outside the audio loop (`crates/foxcrush/src/lib.rs:123-134`).
- Correct: the audio callback keeps parameter reads and per-sample work explicit and simple: smoothed parameter values are read once per sample frame (`crates/foxcrush/src/lib.rs:149-153`), then each channel is processed with per-channel state (`crates/foxcrush/src/lib.rs:155-164`).
- Correct: the custom knob is already factored into a reusable widget module inside the crate (`crates/foxcrush/src/widgets/mod.rs:1-3`) and depends mainly on nih-plug/Vizia abstractions rather than Foxcrush-specific DSP.

## Prioritized recommendations

### 1. High impact / medium risk: split shared UI widgets and theme primitives into a reusable crate

Evidence:
- The workspace currently contains only `crates/foxcrush` and a nested `crates/foxcrush/xtask` (`Cargo.toml:3-7`).
- The custom `ParamKnob` widget is generic over `nih_plug::prelude::Param` (`crates/foxcrush/src/widgets/knob.rs:33-38`) and is re-exported only inside Foxcrush (`crates/foxcrush/src/widgets/mod.rs:1-3`).
- The editor imports `crate::widgets::{ParamKnob, ParamKnobExt}` (`crates/foxcrush/src/editor.rs:6`) and then provides Foxcrush-specific layout around it (`crates/foxcrush/src/editor.rs:88-106`).

Recommendation:
- Create a future shared crate such as `crates/foxplug-ui` once a second plugin is started, and move generic pieces there first: `ParamKnob`, `ParamKnobExt` or its replacement, shared style class names, drawing helpers such as `draw_arc()`, and theme/color constants.
- Keep plugin-specific layout, title text, and parameter selection in `foxcrush/src/editor.rs`.

Why this helps:
- Future plugins can reuse the knob without depending on the `foxcrush` plugin crate.
- The current widget file is ~447 lines and includes control behavior, text input, snapping, and drawing in one place (`crates/foxcrush/src/widgets/knob.rs:20-447`), making it the clearest candidate for sharing.

Risk notes:
- Do not move the entire editor yet. `editor::create()` is Foxcrush-specific because it takes `Arc<FoxcrushParams>` and lays out exactly four parameters (`crates/foxcrush/src/editor.rs:81-107`).

### 2. High impact / low risk: move parameter definitions/builders out of `lib.rs`

Evidence:
- `lib.rs` currently contains the plugin struct, parameter struct, parameter defaults, plugin trait implementation, CLAP/VST3 metadata, formatter helper, and export macros in one file (`crates/foxcrush/src/lib.rs:11-201`).
- Parameter definitions occupy `crates/foxcrush/src/lib.rs:16-94`; plugin integration starts immediately after at `crates/foxcrush/src/lib.rs:96`.
- Common future-plugin parameters already exist: dry/wet `mix` (`crates/foxcrush/src/lib.rs:73-77`) and `output_gain` (`crates/foxcrush/src/lib.rs:79-91`).

Recommendation:
- Add a `params.rs` module for `FoxcrushParams`, `Default`, and parameter formatter helpers such as `smart_rounded()` (`crates/foxcrush/src/lib.rs:190-198`).
- Consider small reusable parameter builder helpers later, e.g. `mix_param(default)`, `output_gain_db_param(min_db, max_db)`, and `smart_rounded_formatter()`, but keep Foxcrush-specific IDs/names near `FoxcrushParams`.

Why this helps:
- Keeps `lib.rs` focused on plugin lifecycle and host integration.
- Reduces copy/paste when future plugins need standard mix/output controls.

Risk notes:
- This is mostly file movement and import cleanup. Public parameter IDs must remain unchanged (`bit_depth`, `downsample`, `mix`, `output_gain` at `crates/foxcrush/src/lib.rs:21-31`) to preserve host automation/session compatibility.

### 3. High impact / medium risk: introduce a processor object to own DSP state and audio-process policy

Evidence:
- Per-channel state is owned directly by `Foxcrush` (`crates/foxcrush/src/lib.rs:11-14`).
- `initialize()` sizes `channel_states` from the main output channel count (`crates/foxcrush/src/lib.rs:129-133`).
- `process()` directly indexes `self.channel_states[ch_idx]` (`crates/foxcrush/src/lib.rs:155-162`) and applies dry/wet/output gain in the plugin layer (`crates/foxcrush/src/lib.rs:163`).
- The docs describe DSP as testable in isolation (`crates/foxcrush/docs/architecture.md:98-103`), but the complete effect behavior currently spans both `dsp.rs` and `Plugin::process()`.

Recommendation:
- Add a `processor.rs` or `dsp::Bitcrusher` type that owns `Vec<BitcrushChannelState>` and exposes `prepare(num_channels)`, `reset()`, and `process_sample/channel_frame(...)` methods.
- Pass a small plain settings struct per sample or block, e.g. `BitcrusherParams { bit_depth, downsample, mix, output_gain }`, so nih-plug parameter reads stay in `Plugin::process()` but DSP policy is testable without host types.
- Add a debug assertion or graceful resize/skip path for channel-state mismatch before indexing.

Why this helps:
- Makes the plugin lifecycle boundary clearer: nih-plug integration reads params and buffers; processor owns effect state and math.
- Makes later plugins easier to model with a common `prepare/reset/process` pattern.

Risk notes:
- Keep realtime constraints: no allocation in `process()`. Any resizing should remain in `initialize()`/`prepare()` or be guarded outside the audio hot path.

### 4. High impact / medium risk: make DSP tests build without the GUI stack

Evidence:
- No test files were found in the repository.
- `dsp.rs` is pure and small enough to test directly (`crates/foxcrush/src/dsp.rs:1-28`).
- GUI dependencies are unconditional in `crates/foxcrush/Cargo.toml:15-17`, so even logic tests build Vizia/Skia.
- `cargo test --workspace` could not complete in this environment because `skia-bindings` attempted to download binaries, received HTTP 403, then attempted a full source unpack into a read-only Cargo registry path.

Recommendation:
- Add unit tests for `BitcrushChannelState::reset()`, downsample hold behavior, and quantization edge cases inside `dsp.rs`.
- Gate editor/widget code behind a feature such as `gui`, or split pure DSP into a small crate (`foxcrush-dsp`) with no Vizia/Skia dependency.
- Keep the plugin crate enabling GUI for normal plugin builds, but allow `cargo test -p foxcrush --lib --no-default-features` or `cargo test -p foxcrush-dsp` to verify DSP quickly.

Why this helps:
- Gives fast feedback for future DSP refactors and avoids GUI native dependency failures blocking pure logic tests.

Risk notes:
- Feature-gating must still preserve normal CLAP/VST3 builds with editor support unless an explicit headless build is desired.

### 5. Medium impact / low risk: centralize workspace dependencies and future-proof `xtask`

Evidence:
- `nih_plug` is declared in the plugin crate (`crates/foxcrush/Cargo.toml:16`) and `nih_plug_xtask` separately in the nested xtask (`crates/foxcrush/xtask/Cargo.toml:8-9`).
- The workspace has `[workspace.package]` but no `[workspace.dependencies]` (`Cargo.toml:9-15`).
- The cargo alias invokes a package named `xtask` (`.cargo/config.toml:1-2`). If each future plugin gets its own nested package also named `xtask`, package selection will become ambiguous.

Recommendation:
- Move shared dependency declarations to `[workspace.dependencies]` when adding more crates/plugins.
- Prefer one workspace-level `xtask` crate, e.g. `crates/xtask` or `xtask`, that can bundle any plugin in the workspace.
- Consider pinning git dependencies to explicit revisions in manifests if reproducibility outside `Cargo.lock` matters.

Why this helps:
- Avoids repeated dependency configuration and package-name collisions as the monorepo grows.

Risk notes:
- Low risk once done before adding a second plugin; higher churn if delayed until several crates exist.

### 6. Medium impact / medium risk: refine the reusable knob API before publishing it as shared API

Evidence:
- The current knob API is `ParamKnob::new(cx, param).snap_to(step)` (`crates/foxcrush/src/widgets/knob.rs:33-38`, `crates/foxcrush/src/widgets/knob.rs:134-141`).
- Snapping is expressed as `Option<f32>` in plain parameter units (`crates/foxcrush/src/widgets/knob.rs:190-199`) and used for scroll stepping as well (`crates/foxcrush/src/widgets/knob.rs:202-225`).
- Bipolar rendering is inferred from whether the default normalized value is near the center (`crates/foxcrush/src/widgets/knob.rs:166-168`).
- Visual constants are hardcoded in the widget (`crates/foxcrush/src/widgets/knob.rs:9-18`, `crates/foxcrush/src/widgets/knob.rs:347-349`).

Recommendation:
- Before moving this to a shared crate, replace the extension-trait mutator with an options/config object, e.g. `ParamKnobOptions { snap_step, bipolar, diameter, drag_pixels_per_range }`.
- Make bipolar mode explicit or parameterized instead of inferred from the default value.
- Move colors and dimensions into theme/config where practical.

Why this helps:
- A shared UI crate should avoid baking Foxcrush’s visual style and heuristics into all future plugins.

Risk notes:
- This is a public API shape decision. Do it before multiple plugins depend on the current `.snap_to()` API.

### 7. Medium impact / low risk: document the intended public API boundaries

Evidence:
- The crate is built both as `cdylib` and `lib` (`crates/foxcrush/Cargo.toml:9-10`).
- `Foxcrush` and `FoxcrushParams` are public (`crates/foxcrush/src/lib.rs:11-17`), and individual params are public (`crates/foxcrush/src/lib.rs:21-31`).
- Internals such as `dsp`, `editor`, and `widgets` are private modules (`crates/foxcrush/src/lib.rs:5-7`), even though the widget is the most reusable code.

Recommendation:
- Decide whether `foxcrush` is only a plugin artifact or also a library API.
- If it is only a plugin artifact, keep internals private and place reusable APIs in dedicated shared crates.
- If it is meant to be imported by tooling/tests/examples, expose only stable modules intentionally and document them.

Why this helps:
- Prevents future plugins from depending on accidental public types such as `FoxcrushParams` while being unable to access actually reusable internals.

Risk notes:
- Public parameter fields may be convenient for the editor, but public API compatibility becomes harder after release.

### 8. Low impact / low risk: update docs so future refactors do not follow stale architecture notes

Evidence:
- `crates/foxcrush/README.md` says “VST/LV2/CLAP” (`crates/foxcrush/README.md:1-3`), but architecture docs explicitly list LV2 as not supported (`crates/foxcrush/docs/architecture.md:16-18`, `crates/foxcrush/docs/architecture.md:111-118`).
- The architecture doc says v1 uses sliders and custom rotary knobs are future work (`crates/foxcrush/docs/architecture.md:22-24`, `crates/foxcrush/docs/architecture.md:67-74`), while the code now uses a custom `ParamKnob` in the editor (`crates/foxcrush/src/editor.rs:92-100`).

Recommendation:
- Update `crates/foxcrush/README.md` and `crates/foxcrush/docs/architecture.md` after the refactor direction is chosen.
- Keep docs aligned with the actual module boundaries and which pieces are intended to become shared crates.

Why this helps:
- Prevents stale docs from misleading future plugin work.

## Suggested sequencing

1. Add DSP tests and/or a headless DSP test path first; this protects behavior during refactors.
2. Move `FoxcrushParams` into `params.rs` and introduce small parameter builder helpers.
3. Introduce a `Bitcrusher`/processor object that owns channel state and preserves no-allocation audio-thread behavior.
4. Once a second plugin is imminent, create `foxplug-ui` and move the knob/theme primitives there with a cleaned-up options API.
5. Promote `xtask` and shared dependencies to workspace-level conventions before adding multiple plugin crates.
6. Update docs last to reflect the new boundaries.
