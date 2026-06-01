# Foxcrush real-time audio/Rust performance review

Scope: reviewed the current `crates/foxcrush` Rust code from a real-time audio, DSP state, parameter, and plugin-practice perspective. The requested `/home/foxfire/code/foxplugs/plan.md` and `/home/foxfire/code/foxplugs/progress.md` files were not present, so this review is based on the repository source/docs. Verification run: `cargo test -p foxcrush` passed, but reported 0 tests.

## Correct / already good

- **No obvious heap allocation or locking in `Plugin::process()`.** The audio loop in `crates/foxcrush/src/lib.rs:149-165` reads smoothed params, processes samples, and writes results. The only DSP state allocation I found is in `initialize()` (`vec![...]` at `crates/foxcrush/src/lib.rs:129-133`), outside the audio callback.
- **Per-channel DSP state is small and cache-friendly.** `BitcrushChannelState` is two `f32`s (`held`, `counter`) in `crates/foxcrush/src/dsp.rs:1-5`, reset explicitly at `crates/foxcrush/src/dsp.rs:7-11`.
- **Parameter smoothing is applied before use in the audio loop.** `bit_depth`, `downsample`, `mix`, and `output_gain` all call `.smoothed.next()` once per sample frame in `crates/foxcrush/src/lib.rs:149-153`; mix/output gain smoothing is defined at `crates/foxcrush/src/lib.rs:73-91`.
- **The editor does not directly touch DSP state.** UI creation receives `Arc<FoxcrushParams>` (`crates/foxcrush/src/editor.rs:81-87`) and uses parameter widgets/events rather than sharing `channel_states`.
- **Allocation checking is enabled for development.** `nih_plug` is built with `assert_process_allocs` in `crates/foxcrush/Cargo.toml:15-17`, which is useful for catching accidental audio-thread allocations in debug builds.

## Prioritized recommendations

### P1 — Avoid an initial/reset sample-hold silence or transient

Evidence:
- `reset()` sets `held = 0.0` and `counter = 0.0` (`crates/foxcrush/src/dsp.rs:7-11`).
- `process_sample()` only updates `held` after `counter >= downsample` (`crates/foxcrush/src/dsp.rs:20-24`).

If `downsample > 1.0` at reset/start, the first processed samples output the old/default held value of `0.0` until the counter reaches the downsample interval. That can create an audible dropout/transient at transport start, preset load, or plugin reset.

Recommendation: make the first sample after reset capture immediately. Common approaches:
- initialize/reset the phase so the next call updates `held`, or
- add a `needs_sample: bool` flag, or
- use an integer countdown initialized to zero.

Add DSP tests for `downsample = 1`, `downsample > 1`, and reset behavior.

### P1 — Move `powf()` out of the per-channel hot path

Evidence:
- `process()` calls `dsp::process_sample()` once per channel per sample frame (`crates/foxcrush/src/lib.rs:155-162`).
- `process_sample()` recomputes `2.0_f32.powf(bit_depth - 1.0)` every call (`crates/foxcrush/src/dsp.rs:26-27`).

`powf()` is expensive for a real-time per-sample/per-channel path, and the computed quantization scale is identical for all channels in a frame.

Recommendation: compute the quantization scale once per sample frame in `process()` after reading smoothed `bit_depth`, then pass `steps`/`scale` into DSP. If bit depth is intended to be discrete, consider an integer/stepped parameter and a small lookup table instead.

### P1 — Remove possible panics from the audio callback indexing path

Evidence:
- Channel state is allocated based on `main_output_channels` in `initialize()` (`crates/foxcrush/src/lib.rs:129-133`).
- The audio callback indexes directly with `self.channel_states[ch_idx]` (`crates/foxcrush/src/lib.rs:155-162`).

With the current declared layout this should normally be stereo (`crates/foxcrush/src/lib.rs:103-107`), but a host/layout mismatch or process-before-initialize bug would panic in the audio callback. Panics in RT audio are effectively catastrophic.

Recommendation: make the invariant impossible or explicitly guarded. Since the plugin currently only advertises stereo, a fixed `[BitcrushChannelState; 2]` layout would avoid the heap allocation and out-of-bounds risk. If future variable channel counts are desired, validate/resize outside processing and make `process()` fail/bypass safely instead of indexing unchecked.

### P2 — Decide whether bit depth/downsample are discrete or continuous parameters

Evidence:
- `bit_depth` and `downsample` are continuous `FloatParam`s (`crates/foxcrush/src/lib.rs:48-71`).
- The UI snaps them only in the custom knob (`crates/foxcrush/src/editor.rs:92-95`), but host automation/text input can still produce arbitrary continuous values.
- DSP uses fractional values directly: `downsample` controls the sample-hold threshold (`crates/foxcrush/src/dsp.rs:20-24`) and `bit_depth` feeds `powf()` (`crates/foxcrush/src/dsp.rs:26`).

If this is intended to model bit depth and downsample factors, integer/stepped values are more idiomatic and more predictable for users/hosts. Fractional downsample factors produce alternating hold periods, and fractional bit depth makes the label “bits” less literal.

Recommendation: either:
- convert these to `IntParam`/stepped parameters and remove smoothing where stepping is musically intentional, or
- explicitly treat them as continuous creative controls and update display/labels/docs accordingly.

### P2 — Add DSP and plugin-invariant tests

Evidence: `cargo test -p foxcrush` passes but reports 0 unit/doc tests.

Recommended tests:
- quantization outputs at known bit depths;
- `downsample = 1` is transparent except quantization;
- reset/start captures the first sample correctly;
- downsample changes do not leave stale `held` values unexpectedly;
- process/channel-state invariant for the supported stereo layout.

These tests are especially valuable before optimizing `process_sample()` because they lock down the intended “Redux-like but not exact clone” behavior.

### P3 — Gate `assert_process_allocs` behind a development feature

Evidence: `assert_process_allocs` is always enabled in `crates/foxcrush/Cargo.toml:15-17`.

This is good during development, but for distributable debug/profiling builds it can turn an accidental allocation into a process panic. Consider exposing it as a repo feature such as `dev-assert-process-allocs` instead of enabling it unconditionally.

### P3 — Consider fixed stereo state or a small-vector state container

Evidence:
- The plugin advertises only stereo I/O (`crates/foxcrush/src/lib.rs:103-107`).
- DSP state is stored in a heap-backed `Vec` (`crates/foxcrush/src/lib.rs:11-14`) allocated in `initialize()` (`crates/foxcrush/src/lib.rs:129-133`).

The current allocation is outside the audio path and not a correctness problem under normal host behavior. But for a fixed-stereo v1, `[BitcrushChannelState; 2]` is simpler, avoids heap state, and makes channel-state cardinality explicit.

### P3 — UI drawing allocates/builds paths, but not on the audio thread

Evidence:
- The custom knob constructs `vg::PathBuilder`s and snapshots during draw (`crates/foxcrush/src/widgets/knob.rs:404-413`, `crates/foxcrush/src/widgets/knob.rs:427-446`).

This is not an audio-thread issue. Only revisit if UI redraw performance becomes a problem; the audio callback is isolated from this work.

## Summary priority order

1. Fix reset/first-sample sample-hold behavior.
2. Compute quantization scale once per frame or use discrete lookup.
3. Make channel-state indexing panic-proof in the audio callback.
4. Decide and encode discrete vs continuous parameter semantics for bit depth/downsample.
5. Add DSP tests before/alongside refactoring.
6. Gate development-only allocation assertions and consider fixed stereo state as cleanup.
