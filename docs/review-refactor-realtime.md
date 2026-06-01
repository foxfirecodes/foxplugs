# Review — Foxcrush/Foxplugs refactor real-time audio safety

Input note: `/home/foxfire/code/foxplugs/plan.md` and `/home/foxfire/code/foxplugs/progress.md` were not present when requested. I reviewed the current uncommitted diff, `docs/refactor.md`, and the implementation report instead.

## Review

- Correct: The `process()` hot path is a thin adapter and does not show evidence of heap allocation, locks, logging, syscalls, unwrap/expect panics, or trait-object dispatch inside the per-sample loop. It reads smoothed params once per frame, builds a stack `BitcrusherFrameParams`, then calls the concrete `Bitcrusher::process_frame()` (`crates/foxcrush/src/lib.rs:56-74`). The processor loop uses iterator `zip()` over mutable samples and fixed channel state with only arithmetic and DSP calls (`crates/foxcrush/src/processor.rs:39-50`).

- Correct: The previous channel-index panic risk is addressed for the fixed stereo design. The plugin advertises two input/output channels from `Bitcrusher::CHANNELS` (`crates/foxcrush/src/lib.rs:26-30`), and the processor owns `[BitcrushChannelState; 2]` (`crates/foxcrush/src/processor.rs:25-31`). Processing uses `samples.into_iter().zip(self.channel_states.iter_mut())` instead of indexing by channel number (`crates/foxcrush/src/processor.rs:45-49`). The test `extra_channels_do_not_panic_or_get_processed` covers the over-channel case (`crates/foxcrush/src/processor.rs:115-126`).

- Correct: Reset/sample-hold behavior now captures the first sample immediately after default construction or reset. `BitcrushChannelState::default()` and `reset()` both set `needs_sample = true` (`crates/foxcrush/src/dsp.rs:8-23`), and `process_sample()` captures input before advancing the counter when `needs_sample` is set (`crates/foxcrush/src/dsp.rs:49-58`). DSP and processor tests cover this (`crates/foxcrush/src/dsp.rs:159-179`, `crates/foxcrush/src/processor.rs:97-112`).

- Correct: `powf()` has been moved out of the per-channel DSP function. `dsp::process_sample()` receives precomputed `quantization_steps` (`crates/foxcrush/src/dsp.rs:41-61`), while `BitcrusherFrameParams::from_plain_values()` computes it once for the sample frame (`crates/foxcrush/src/processor.rs:13-21`) after the plugin reads smoothed parameters (`crates/foxcrush/src/lib.rs:62-68`).

- Correct: The continuous bit-depth/downsample decision is consistently represented in code and tests. Both remain `FloatParam`s with smoothing (`crates/foxcrush/src/params.rs:45-72`), DSP documents fractional downsample behavior (`crates/foxcrush/src/dsp.rs:36-39`), and tests cover continuous bit depth and fractional downsample (`crates/foxcrush/src/dsp.rs:88-92`, `crates/foxcrush/src/dsp.rs:136-157`).

- Correct: UI extraction is isolated from no-GUI/audio builds. `foxplugs-ui` and `vizia_plug` are optional behind the `gui` feature (`crates/foxcrush/Cargo.toml:12-20`), `editor` is `#[cfg(feature = "gui")]` (`crates/foxcrush/src/lib.rs:4-6`, `crates/foxcrush/src/lib.rs:42-50`), and the audio `process()` path has no dependency on `foxplugs-ui` (`crates/foxcrush/src/lib.rs:56-74`). The knob snap semantics also remain UI-only and in plain parameter units (`crates/foxplugs-ui/src/widgets/knob.rs:20-24`, `crates/foxplugs-ui/src/widgets/knob.rs:229-236`), with Foxcrush preserving linear output-gain snapping (`crates/foxcrush/src/editor.rs:30-33`).

- Correct: DSP/processor test coverage is meaningful for the refactor areas: quantization, negative samples, downsample=1, downsample>1, fractional downsample, reset capture, fixed stereo count, silence, mix=0, and extra channels are covered in `crates/foxcrush/src/dsp.rs:77-179` and `crates/foxcrush/src/processor.rs:67-126`.

- Note [Low / performance]: `powf()` is no longer per-channel, but it is still in the audio hot path once per sample frame through `BitcrusherFrameParams::from_plain_values()` (`crates/foxcrush/src/processor.rs:15-21`) and `dsp::quantization_steps()` (`crates/foxcrush/src/dsp.rs:26-29`). This matches the refactor plan’s “once per sample frame” target, but if profiling shows CPU pressure, a lookup/approximation or different smoothing strategy could remove `powf()` from the per-frame path entirely.

- Note [Low / test gap]: The `dev-assert-process-allocs` feature is correctly gated (`crates/foxcrush/Cargo.toml:12-20`) and builds/tests pass with it enabled, but the current unit tests exercise the pure DSP/processor layer rather than a full `Plugin::process()` call under NIH’s allocation guard. Given the inspected hot path, I do not see an allocation issue; this is only a possible future regression test improvement.

- Blocker: None found from a real-time audio performance/safety perspective.

## Validation run

- `cargo test -p foxcrush --lib --no-default-features` — passed, 11 tests.
- `cargo check -p foxcrush --no-default-features --features dev-assert-process-allocs` — passed.
- `cargo test -p foxcrush --lib` — passed, 11 tests.
- `cargo test -p foxcrush --lib --no-default-features --features dev-assert-process-allocs` — passed, 11 tests.
