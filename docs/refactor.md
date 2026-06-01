# Foxplugs/Foxcrush Refactor Plan

This plan captures the full set of refactoring recommendations from the local review and the two reviewer reports:

- `foxcrush-refactor-architecture.md`
- `foxcrush-refactor-performance.md`

Goal: make Foxcrush cleaner and safer for real-time audio work while preparing the workspace to support multiple future Foxplugs plugins with shared DSP, UI, build, and plugin-integration infrastructure.

## Guiding principles

1. **Protect audio-thread safety first.** No allocations, locks, syscalls, panics, logging, or unbounded work in `Plugin::process()` or other real-time callbacks.
2. **Test DSP behavior before reshaping it.** Add behavior tests before changing implementation details so refactors preserve intentional sound.
3. **Keep DSP independent from plugin/UI frameworks.** DSP should not depend on NIH-plug, Vizia, editor state, or host types.
4. **Use zero-cost Rust patterns in hot paths.** Prefer concrete structs, arrays, slices, generics, and plain data over trait objects or dynamic dispatch inside audio processing.
5. **Make shared crates intentional.** Future plugins should depend on `foxplugs-*` crates, not on accidental internals from the `foxcrush` plugin crate.
6. **Preserve host compatibility.** Existing parameter IDs must remain stable unless we intentionally decide to break old sessions/presets before release.
7. **Keep docs current.** Architecture docs should describe the actual module boundaries and supported plugin formats.

## Target workspace shape

The eventual structure should move reusable code out of the Foxcrush plugin crate:

```text
crates/
  foxcrush/             # Foxcrush plugin: metadata, params, editor layout, processor wiring
  foxplugs-dsp/         # shared DSP utilities and test helpers, no GUI/plugin deps
  foxplugs-ui/          # reusable Vizia/NIH-plug widgets, theme, layout helpers
  foxplugs-plugin/      # shared NIH-plug param/build helpers, metadata conventions
  xtask/                # workspace-level bundling/dev task crate
```

This does not all need to happen in one commit. The first milestones should make Foxcrush internally cleaner, then extract shared crates when the seams are stable.

## Phase 1 — Add tests and lock down current DSP behavior

### 1.1 Add DSP unit tests

Add unit tests in `crates/foxcrush/src/dsp.rs` or a dedicated DSP test module.

Test cases:

- quantization outputs at known bit depths;
- negative sample quantization behavior;
- `downsample = 1.0` captures every sample;
- `downsample > 1.0` holds samples for the expected duration;
- reset/start behavior captures the first input sample instead of outputting stale/default silence;
- reset clears channel state deterministically.

Acceptance:

- `cargo test -p foxcrush --lib` runs meaningful DSP tests.
- The tests document whether bit depth/downsample are currently treated as continuous or discrete controls.

### 1.2 Add plugin/DSP invariant tests where practical

Add focused tests for invariants that should never break:

- supported channel-state count matches the advertised stereo layout;
- processor reset does not allocate;
- processing silence remains silence;
- mix/output gain behavior is predictable at `mix = 0`, `mix = 1`, and unity gain.

Acceptance:

- Invariants are testable without launching a DAW.
- Tests do not require the GUI stack where avoidable.

## Phase 2 — Fix real-time correctness/performance issues

### 2.1 Fix initial sample-hold transient after reset

Current issue: `BitcrushChannelState::reset()` sets `held = 0.0` and `counter = 0.0`, while `process_sample()` only captures new input after `counter >= downsample`. With `downsample > 1.0`, early samples after reset can output `0.0`.

Options:

- add `needs_sample: bool` and capture immediately on first process call;
- initialize phase/counter so the next call captures immediately;
- convert to a countdown model initialized to zero.

Preferred direction: use an explicit state field such as `needs_sample` or a countdown because it makes the reset behavior obvious.

Acceptance:

- First sample after reset is based on the first input sample.
- Tests cover reset/start behavior for `downsample = 1.0` and `downsample > 1.0`.

### 2.2 Move `powf()` out of the per-channel hot path

Current issue: `dsp::process_sample()` recomputes `2.0_f32.powf(bit_depth - 1.0)` per channel per sample.

Plan:

- Compute quantization scale once per sample frame after reading the smoothed `bit_depth` value.
- Pass the computed scale/steps into DSP.
- If bit depth becomes discrete, consider a lookup table or integer shift-like scale computation later.

Acceptance:

- `powf()` is not called once per channel.
- DSP tests still pass.

### 2.3 Make channel-state access panic-proof

Current issue: `Plugin::process()` indexes `self.channel_states[ch_idx]` directly.

Options:

- for fixed stereo v1, replace `Vec<BitcrushChannelState>` with `[BitcrushChannelState; 2]`;
- if variable channel counts are desired later, keep a `Vec` but validate/prepare outside the hot path and use safe iteration patterns such as `zip()`.

Preferred direction for current Foxcrush: fixed stereo state, because the plugin advertises only stereo.

Acceptance:

- No out-of-bounds indexing path exists in the audio callback for the supported layout.
- Any future variable-channel implementation must validate state size before processing.

### 2.4 Decide and encode bit depth/downsample semantics

Current state:

- `bit_depth` and `downsample` are continuous `FloatParam`s.
- The custom UI snaps them, but host automation/text entry can still produce arbitrary continuous values.
- DSP currently accepts fractional values.

Decision to make:

- **Discrete model:** use `IntParam` or stepped `FloatParam`; avoid smoothing for hard steps if that is musically intended; host automation reflects valid values.
- **Continuous model:** keep `FloatParam`, document fractional behavior, and label controls accordingly.

Recommended direction:

- Treat `downsample` as a stepped/integer factor unless we intentionally want fractional sample-hold rhythms.
- Treat `bit_depth` as stepped if the label remains “bits.” Half-bit UI snapping should be reconsidered or explicitly documented as creative behavior.

Acceptance:

- Parameter type/step metadata matches intended behavior for both custom UI and host generic UI.
- Documentation explains the behavior.

### 2.5 Review development-only allocation assertions

Current state: `nih_plug` enables `assert_process_allocs` unconditionally.

Plan:

- Keep allocation checking available during development.
- Consider gating it behind a feature such as `dev-assert-process-allocs` before distribution-oriented debug/profiling builds.

Acceptance:

- Normal developer builds can still catch accidental process allocations.
- Release/distribution behavior is intentional and documented.

## Phase 3 — Clean Foxcrush internal module boundaries

### 3.1 Move parameters into `params.rs`

Current issue: `crates/foxcrush/src/lib.rs` contains plugin state, params, defaults, metadata, formatters, and exports.

Plan:

- Add `crates/foxcrush/src/params.rs`.
- Move `FoxcrushParams`, `Default for FoxcrushParams`, and parameter-specific formatter helpers such as `smart_rounded()`.
- Keep parameter IDs unchanged:
  - `bit_depth`
  - `downsample`
  - `mix`
  - `output_gain`

Acceptance:

- `lib.rs` focuses on plugin lifecycle and format exports.
- Parameter IDs are preserved.

### 3.2 Introduce a processor object

Current issue: `Foxcrush` directly owns channel state and implements effect policy inside `Plugin::process()`.

Plan:

- Add `crates/foxcrush/src/processor.rs`.
- Introduce a concrete processor type, for example:

```rust
pub struct Bitcrusher {
    channel_states: [BitcrushChannelState; 2],
}

pub struct BitcrusherFrameParams {
    pub bit_depth: f32,
    pub downsample: f32,
    pub mix: f32,
    pub output_gain: f32,
}
```

- Processor owns `prepare`, `reset`, and frame/sample processing policy.
- NIH-plug parameter reads remain in `Plugin::process()`.
- DSP math remains in `dsp.rs` or under a `dsp` module.

Acceptance:

- `Foxcrush::process()` is a thin adapter from NIH-plug buffer/params to processor calls.
- DSP/effect behavior can be tested without host/editor types.
- Processor does not allocate in `process()`.

### 3.3 Clarify public API boundaries

Current state:

- `Foxcrush` and `FoxcrushParams` are public because the crate also builds as `lib`.
- `dsp`, `editor`, and `widgets` are private modules.

Plan:

- Decide that `foxcrush` is primarily a plugin artifact, not a shared API crate.
- Keep plugin internals private unless tests/examples need them.
- Move truly reusable APIs into shared crates rather than exporting them from `foxcrush`.

Acceptance:

- Future plugins do not depend on `foxcrush` internals.
- Shared code lives in intentionally named `foxplugs-*` crates.

## Phase 4 — Extract reusable plugin helpers

### 4.1 Add shared parameter/helper module or crate

Reusable candidates:

- `mix_param(default)`;
- `output_gain_db_param(default_db, min_db, max_db)`;
- `smart_rounded_formatter()`;
- standard smoothing durations/constants;
- common percentage/dB formatter wrappers;
- optional plugin metadata conventions.

Initial location options:

- keep private helper module inside `foxcrush` during experimentation;
- extract to `crates/foxplugs-plugin` once a second plugin needs it.

Acceptance:

- Common mix/output parameter construction is not copy/pasted across plugins.
- Parameter IDs/names remain plugin-local and explicit.

### 4.2 Avoid over-abstracting plugin processing

Do not introduce a generic boxed `AudioProcessor` trait used dynamically in the audio callback unless there is a compelling reason.

Preferred patterns:

- concrete processor structs;
- plain parameter snapshots;
- generic helper functions outside the innermost loops;
- monomorphized traits only when they do not add runtime cost.

Acceptance:

- Shared helpers improve consistency without adding dynamic dispatch or heap allocation in hot paths.

## Phase 5 — Extract reusable UI primitives

### 5.1 Refine `ParamKnob` API before extraction

Current API:

```rust
ParamKnob::new(cx, param).snap_to(Some(step))
```

Issues:

- snapping is a bare `Option<f32>` in plain units;
- bipolar rendering is inferred from the default normalized value;
- colors/dimensions are hardcoded;
- drawing helpers are embedded in the widget file.

Plan:

- Introduce an explicit options/config type, for example:

```rust
pub struct ParamKnobOptions {
    pub snap_step: Option<f32>,
    pub bipolar: Option<bool>,
    pub diameter: f32,
    pub drag_pixels_per_full_range: f32,
}
```

- Make bipolar mode explicit or caller-configurable.
- Move visual constants/colors toward a theme/config layer.
- Keep unsafe `ParamPtr` usage isolated inside the widget module and document safety assumptions.

Acceptance:

- The knob API is suitable for reuse by multiple plugins.
- Foxcrush visual styling is not hardcoded as the only possible style.

### 5.2 Create `foxplugs-ui`

When the API is stable enough, create `crates/foxplugs-ui`.

Move or expose:

- `ParamKnob`;
- knob options/config;
- shared theme colors/style classes;
- drawing helpers such as arc drawing;
- optional layout helpers for knob grids/rows/cells.

Keep plugin-specific UI in Foxcrush:

- title text;
- exact parameter selection;
- parameter order;
- plugin-specific dimensions if desired.

Acceptance:

- `foxcrush/src/editor.rs` imports shared UI primitives from `foxplugs-ui`.
- Future plugins can use knobs/theme without depending on `foxcrush`.

### 5.3 Review output-gain snapping behavior

Current state: output gain is stored as linear gain but UI snap step is `Some(0.05)`, which means snapping occurs in linear gain units, not dB.

Plan:

- Decide whether output gain should snap in dB or linear gain.
- If dB snapping is desired, either store the parameter in dB or add a display/domain snapping strategy to the knob options.

Acceptance:

- Output gain snapping behavior matches user expectations and documentation.

## Phase 6 — Workspace/build cleanup

### 6.1 Move `xtask` to workspace level

Current state: `xtask` lives at `crates/foxcrush/xtask` and the cargo alias invokes a package named `xtask`.

Plan:

- Move to `crates/xtask` or top-level `xtask`.
- Update `Cargo.toml` workspace members.
- Update `.cargo/config.toml` alias if needed.
- Ensure the bundler can bundle `foxcrush` and future plugins.

Acceptance:

- There is one workspace-level `xtask` package.
- Adding a second plugin does not create package-name collisions.

### 6.2 Add `[workspace.dependencies]`

Current state: shared dependencies are declared per crate.

Plan:

- Add workspace dependency declarations for shared dependencies such as:
  - `nih_plug`
  - `vizia_plug`
  - `nih_plug_xtask`
- Update member crates to use `{ workspace = true }`.
- Consider pinning git dependencies to explicit revisions for reproducibility.

Acceptance:

- Shared dependency versions/sources are centralized.
- Future plugin crates can opt into consistent versions.

### 6.3 Consider feature-gating GUI/headless tests

Current issue: pure DSP tests can be slowed or blocked by GUI dependencies such as Vizia/Skia.

Options:

- split DSP into `foxplugs-dsp` or `foxcrush-dsp` with no GUI deps;
- add a `gui` feature around editor/widgets;
- keep plugin builds GUI-enabled by default while allowing fast DSP-only tests.

Acceptance:

- There is a fast test path for DSP logic that does not require building the full GUI stack.

## Phase 7 — Extract shared DSP crate when useful

### 7.1 Create `foxplugs-dsp`

Move or add generic DSP utilities only when a second plugin or reusable test helper needs them.

Potential contents:

- sample/frame helper types;
- denormal-safe utility helpers if needed, while relying on NIH-plug where appropriate;
- quantization helpers;
- dry/wet mixing helpers;
- gain helpers if not kept in plugin-helper crate;
- test utilities for approximate float comparisons and short-buffer processing.

Avoid:

- plugin framework types;
- Vizia/editor dependencies;
- boxed runtime processor abstractions in hot paths.

Acceptance:

- Shared DSP crate builds and tests without GUI/plugin framework dependencies.
- Foxcrush DSP uses shared helpers only where they clarify code without adding overhead.

## Phase 8 — Documentation updates

### 8.1 Update root and crate READMEs

Current issue: `crates/foxcrush/README.md` mentions VST/LV2/CLAP while architecture docs say LV2 is unsupported.

Plan:

- Update supported formats consistently.
- Document build/test commands.
- Mention shared crates once they exist.

Acceptance:

- README format/support claims match actual exports.

### 8.2 Update Foxcrush architecture docs

Current issue: `crates/foxcrush/docs/architecture.md` still refers to sliders/custom knobs as future work, but the code already uses a custom knob.

Plan:

- Update diagrams/module descriptions after internal refactors.
- Document processor/params/editor boundaries.
- Document real-time constraints and how the code enforces them.
- Document parameter stepping/continuous decisions.

Acceptance:

- Architecture docs match current code and intended future shared-crate boundaries.

## Implementation checklist

Recommended order:

1. Add DSP tests.
2. Fix reset/first-sample sample-hold behavior.
3. Move quantization-scale calculation out of per-channel processing.
4. Make channel-state access panic-proof, likely with fixed stereo state for Foxcrush v1.
5. Decide/encode bit depth and downsample parameter semantics.
6. Split `params.rs` out of `lib.rs`.
7. Add `processor.rs` and make `Plugin::process()` a thin adapter.
8. Add reusable parameter helper functions privately, then extract later if needed.
9. Refine `ParamKnob` options/config API.
10. Extract `foxplugs-ui`.
11. Move `xtask` to workspace level.
12. Add `[workspace.dependencies]` and update crate manifests.
13. Add GUI/headless feature strategy or a pure DSP crate/test path.
14. Extract `foxplugs-dsp` when reuse is concrete.
15. Update README and architecture docs.

## Validation commands

Use the best available subset after each phase:

```bash
cargo fmt --all --check
cargo test -p foxcrush --lib
cargo test --workspace
cargo xtask bundle foxcrush --release
```

If GUI/native dependencies make full workspace tests slow or flaky, prioritize the DSP-only test path until the headless/shared-DSP split is complete.

## Implementation status

Implemented in the first refactor pass:

- DSP tests for quantization, continuous bit depth, fractional downsample, downsample hold behavior, and reset/first-sample capture.
- Reset/first-sample sample-hold transient fix using explicit `needs_sample` state.
- Quantization scale calculation moved out of the per-channel DSP path and into the per-frame parameter snapshot.
- Fixed stereo processor state with safe iterator zipping instead of channel-state indexing.
- Continuous `bit_depth` and continuous/fractional `downsample` semantics preserved and documented.
- `params.rs` split out of `lib.rs` with private reusable parameter builder helpers.
- `processor.rs` added so `Plugin::process()` is a thin NIH-plug adapter.
- `ParamKnobOptions` added and `ParamKnob` extracted to the new `foxplugs-ui` crate.
- Output gain snapping intentionally preserved in linear gain units.
- Workspace-level `xtask` moved to `crates/xtask`.
- Shared dependencies centralized under `[workspace.dependencies]`.
- GUI is behind the default `gui` feature, enabling a fast `--no-default-features` DSP/processor test path.
- README and architecture docs updated.

Deliberate deferrals:

- `foxplugs-dsp` was not extracted yet because there is no concrete second-plugin DSP reuse. Foxcrush now has a clean DSP/processor seam, and a shared DSP crate should be introduced when reuse is real.
- `foxplugs-plugin` was not extracted yet; common parameter helpers are private in `foxcrush::params` until another plugin needs them.

## Resolved decisions

1. `bit_depth` remains continuous for now.
2. `downsample` remains continuous/fractional for now.
3. Output gain snapping remains in linear gain units.
4. Foxcrush remains stereo-only for v1.
5. `assert_process_allocs` is gated behind the `dev-assert-process-allocs` feature.
