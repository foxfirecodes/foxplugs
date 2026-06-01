# Worker refactor implementation report

## Summary

Implemented the approved Foxcrush/Foxplugs refactor from `docs/refactor.md` with the user's decisions applied:

- `bit_depth` remains continuous.
- `downsample` remains continuous/fractional.
- output gain knob snapping remains in linear gain units.
- Foxcrush remains stereo-only for v1.
- `assert_process_allocs` is now gated behind `dev-assert-process-allocs` instead of being unconditionally enabled.

## Changed files

- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `docs/refactor.md`
- `crates/foxcrush/Cargo.toml`
- `crates/foxcrush/README.md`
- `crates/foxcrush/docs/architecture.md`
- `crates/foxcrush/src/lib.rs`
- `crates/foxcrush/src/dsp.rs`
- `crates/foxcrush/src/params.rs`
- `crates/foxcrush/src/processor.rs`
- `crates/foxcrush/src/editor.rs`
- deleted `crates/foxcrush/src/widgets/knob.rs`
- deleted `crates/foxcrush/src/widgets/mod.rs`
- deleted `crates/foxcrush/xtask/Cargo.toml`
- deleted `crates/foxcrush/xtask/src/main.rs`
- added `crates/foxplugs-ui/Cargo.toml`
- added `crates/foxplugs-ui/src/lib.rs`
- added `crates/foxplugs-ui/src/theme.rs`
- added `crates/foxplugs-ui/src/widgets/mod.rs`
- added `crates/foxplugs-ui/src/widgets/knob.rs`
- added `crates/xtask/Cargo.toml`
- added `crates/xtask/src/main.rs`

## Major design choices

### DSP and processor split

- `dsp.rs` now contains pure quantization/sample-and-hold logic plus tests.
- `processor.rs` owns a fixed stereo `Bitcrusher` processor with `[BitcrushChannelState; 2]`.
- `lib.rs` is now a thin NIH-plug adapter: it reads smoothed params once per frame, builds `BitcrusherFrameParams`, and delegates processing.

### Reset/first-sample fix

`BitcrushChannelState` now has explicit `needs_sample` state. `reset()` sets `needs_sample = true`, and the next `process_sample()` captures the input immediately instead of outputting the previous/default held value.

Evidence:

- `crates/foxcrush/src/dsp.rs`: `needs_sample` field and immediate capture branch in `process_sample()`.
- Tests:
  - `dsp::tests::reset_makes_next_sample_capture_immediately`
  - `processor::tests::reset_captures_first_frame_after_reset`

### Per-channel `powf()` removal

`dsp::process_sample()` no longer computes bit-depth scale. `BitcrusherFrameParams::from_plain_values()` computes `dsp::quantization_steps(bit_depth)` once per sample frame and passes the result into DSP for each channel.

Evidence:

- `crates/foxcrush/src/dsp.rs`: `process_sample()` accepts `quantization_steps`.
- `crates/foxcrush/src/processor.rs`: `BitcrusherFrameParams::from_plain_values()` computes the scale once.
- `crates/foxcrush/src/lib.rs`: one frame params struct is created before processing the channel samples.

### Stereo panic-proofing

Foxcrush v1 uses fixed stereo state. The processor processes samples with iterator `zip()` against the two channel states instead of indexing by channel number. Unexpected extra channels are left unprocessed rather than causing an out-of-bounds panic.

Evidence:

- `crates/foxcrush/src/processor.rs`: `channel_states: [BitcrushChannelState; 2]` and `samples.into_iter().zip(self.channel_states.iter_mut())`.
- Test: `processor::tests::extra_channels_do_not_panic_or_get_processed`.

### UI extraction

The old Foxcrush-local knob module moved into `crates/foxplugs-ui`.

- Added `ParamKnobOptions` with explicit snap step, bipolar mode, diameter, and drag sensitivity.
- Preserved the previous default bipolar heuristic when `bipolar` is `None`.
- Preserved text-entry bypass of snap logic.
- Shared dark stylesheet now lives in `foxplugs-ui::theme`.

### Feature/headless strategy

- `foxcrush` default feature: `gui`.
- `--no-default-features` builds/tests DSP and processor code without compiling `foxplugs-ui`/Vizia.
- `dev-assert-process-allocs` enables `nih_plug/assert_process_allocs` only when requested.

### Deliberate deferrals

- Did not extract `foxplugs-dsp`: there is no concrete second-plugin DSP reuse yet. The pure `dsp.rs` + `processor.rs` seam is ready for extraction later.
- Did not extract `foxplugs-plugin`: parameter helpers are private in `foxcrush::params` until another plugin needs them.

## Validation

Commands run:

1. `cargo fmt --all`
   - Exit code: 0

2. `cargo fmt --all --check`
   - Exit code: 0

3. `cargo test -p foxcrush --lib --no-default-features`
   - Exit code: 0
   - Result: 11 passed, 0 failed

4. `cargo test -p foxcrush --lib`
   - Exit code: 0
   - Result: 11 passed, 0 failed

5. `cargo check -p foxcrush --no-default-features`
   - Exit code: 0

6. `cargo check -p foxcrush --no-default-features --features dev-assert-process-allocs`
   - Exit code: 0

7. `cargo check --workspace --exclude foxplugs-ui --no-default-features`
   - Exit code: 0

8. `cargo test --workspace --exclude foxplugs-ui --no-default-features`
   - Exit code: 0
   - Result: Foxcrush 11 passed; foxcrush bin/xtask/doc tests had 0 tests

9. `cargo test -p foxplugs-ui`
   - Exit code: 101
   - Blocked by `skia-bindings` native dependency download/build in this environment, before compiling crate test code.

10. `cargo test --workspace`
    - Exit code: 101
    - Blocked by the same `skia-bindings` failure.

## Exact blocker excerpt for GUI/workspace tests

The GUI-dependent validation failed because `skia-bindings` could not download the prebuilt binary and then attempted to unpack a source build into a read-only Cargo registry directory:

```text
error: failed to run custom build command for `skia-bindings v0.97.0`

Caused by:
  process didn't exit successfully: `/home/foxfire/code/foxplugs/target/debug/build/skia-bindings-be7887dbc422b9f7/build-script-build` (exit status: 101)
...
  TRYING TO DOWNLOAD AND INSTALL SKIA BINARIES: 0.97.0/fdd89c91b34b4e26dd64-x86_64-unknown-linux-gnu-egl-gl-jpegd-jpege-pdf-svg-textlayout-wayland-x11
  cargo:rerun-if-env-changed=SKIA_BINARIES_URL
    FROM: https://github.com/rust-skia/skia-binaries/releases/download/0.97.0/skia-binaries-fdd89c91b34b4e26dd64-x86_64-unknown-linux-gnu-egl-gl-jpegd-jpege-pdf-svg-textlayout-wayland-x11.tar.gz
  DOWNLOAD AND INSTALL FAILED: curl error code: "22"
  curl stderr: "curl: (22) The requested URL returned error: 403\n"
  STARTING A FULL BUILD
...
  thread 'main' (16) panicked at /home/foxfire/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/skia-bindings-0.97.0/build_support/binary_cache/download.rs:97:43:
  called `Result::unwrap()` on an `Err` value: Custom { kind: ReadOnlyFilesystem, error: TarError { desc: "failed to unpack `/home/foxfire/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/skia-bindings-0.97.0/skia-m148-0.95.2`", io: Custom { kind: ReadOnlyFilesystem, error: "Read-only file system (os error 30) when creating dir /home/foxfire/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/skia-bindings-0.97.0/skia-m148-0.95.2" } } }
```

## Bundle validation

I did not run `cargo xtask bundle foxcrush --release` after the workspace GUI test hit the Skia dependency blocker. The release bundle path uses the default GUI-enabled plugin build, so it is expected to hit the same native dependency issue in this environment unless Skia binaries are available or Cargo can write the Skia source build directory.

## Open risks / next steps

- Run full GUI/workspace tests and the release bundle command in an environment where `skia-bindings` can either download its prebuilt binary or unpack/build Skia.
- When a second plugin starts, revisit extracting `foxplugs-dsp` and `foxplugs-plugin` from the now-clean seams.
