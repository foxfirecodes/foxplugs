# Review — Foxcrush/Foxplugs refactor correctness

Scope note: the requested `/home/foxfire/code/foxplugs/plan.md` and `progress.md` paths were not present (`ENOENT`). I reviewed `docs/refactor.md`, `worker-refactor-implementation.md`, the uncommitted diff, and current source files.

## Review

- Correct (no source blockers found): The refactor matches the resolved user decisions in `docs/refactor.md:504-510`.
  - `bit_depth` remains a continuous `FloatParam` with the original `bit_depth` ID in `crates/foxcrush/src/params.rs:18-19` and continuous range/formatter in `crates/foxcrush/src/params.rs:45-57`.
  - `downsample` remains a continuous/fractional `FloatParam` with the original `downsample` ID in `crates/foxcrush/src/params.rs:21-22` and skewed float range in `crates/foxcrush/src/params.rs:59-72`.
  - Output-gain snapping remains in linear gain units in the Foxcrush UI (`crates/foxcrush/src/editor.rs:30-33`), while the parameter is still stored as linear gain and displayed in dB (`crates/foxcrush/src/params.rs:82-95`).
  - Foxcrush remains stereo-only: advertised layout uses `Bitcrusher::CHANNELS` in `crates/foxcrush/src/lib.rs:26-30`, and the processor uses fixed `[BitcrushChannelState; 2]` state in `crates/foxcrush/src/processor.rs:25-31`.
  - `assert_process_allocs` is no longer unconditional and is gated behind `dev-assert-process-allocs` in `crates/foxcrush/Cargo.toml:12-15`.

- Correct: DSP behavior and hot-path refactor are coherent with `docs/refactor.md`.
  - Reset/start transient is fixed with explicit `needs_sample` state and immediate first-sample capture in `crates/foxcrush/src/dsp.rs:1-23` and `crates/foxcrush/src/dsp.rs:47-61`.
  - Per-channel `powf()` was removed from sample processing. The only production `powf()` is `quantization_steps()` at `crates/foxcrush/src/dsp.rs:26-29`; it is called once per frame in `BitcrusherFrameParams::from_plain_values()` (`crates/foxcrush/src/processor.rs:13-21`) before processing zipped channel samples (`crates/foxcrush/src/processor.rs:45-49`).
  - Channel processing no longer indexes by channel number; `zip()` is used in `crates/foxcrush/src/processor.rs:45`, with a regression test for unexpected extra channels at `crates/foxcrush/src/processor.rs:115-126`.

- Correct: Workspace/build refactor is wired as intended.
  - Workspace members now include `foxcrush`, `foxplugs-ui`, and workspace-level `xtask` in `Cargo.toml:1-4`.
  - Shared dependency declarations are centralized in `Cargo.toml:11-15` and used by `crates/foxcrush/Cargo.toml:17-20`, `crates/foxplugs-ui/Cargo.toml:9-11`, and `crates/xtask/Cargo.toml:9-10`.
  - The moved `xtask` delegates to NIH-plug’s bundler in `crates/xtask/src/main.rs:1-3`.
  - The GUI feature gates `editor` and the no-GUI editor fallback correctly in `crates/foxcrush/src/lib.rs:5-6` and `crates/foxcrush/src/lib.rs:42-50`.

- Correct: Validation that passed in this review.
  - `cargo fmt --all --check`
  - `git diff --check`
  - `cargo check -p foxcrush`
  - `cargo check -p foxcrush --no-default-features`
  - `cargo check -p xtask`
  - `cargo test -p foxcrush --lib` — 11 passed
  - `cargo test -p foxcrush --lib --no-default-features` — 11 passed
  - `cargo test -p foxcrush --features dev-assert-process-allocs` — 11 passed plus 0-test bin/doc targets
  - `cargo test --workspace --exclude foxplugs-ui --no-default-features` — 11 passed plus 0-test bin/xtask/doc targets
  - `cargo xtask bundle foxcrush`
  - `cargo xtask bundle foxcrush --release`

- Blocker: None found in the Rust source, feature gating, parameter behavior, or xtask/workspace wiring.

- Note (Medium, validation environment/build dependency): Full GUI/workspace test validation still fails here before reaching project tests because `skia-bindings v0.97.0` cannot download its prebuilt binary and then attempts to unpack a source build into a read-only Cargo registry directory. This affects commands documented as validation/common commands in `docs/refactor.md:472-477`, `crates/foxcrush/docs/architecture.md:110-114`, and `README.md:13-17`.
  - Reproduced with `cargo test --workspace` and `cargo test -p foxplugs-ui --lib`.
  - Error evidence: `DOWNLOAD AND INSTALL FAILED: curl error code: "22" ... 403`, followed by `ReadOnlyFilesystem ... when creating dir ... skia-bindings-0.97.0/skia-m148-0.95.2`.
  - This does not currently indicate a Foxcrush Rust compile regression: `cargo check -p foxcrush`, GUI-enabled `cargo test -p foxcrush --lib`, and both debug/release `cargo xtask bundle foxcrush` passed.
