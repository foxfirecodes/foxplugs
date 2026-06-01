# Foxshaper Initial Spec

Foxshaper is a ShaperBox-style modulation plugin for the Foxplugs suite. The first milestone is intentionally narrow: **basic volume shaping**. Later phases can generalize the same modulation engine into other shaper lanes.

This spec is based on the ShaperBox 3 manual and the current Foxshaper scaffold. It should guide implementation without copying Cableguys assets, presets, UI artwork, or proprietary algorithms.

## Product shape

- Audio effect plugin, stereo first.
- Built on the shared Foxplugs crates:
  - `foxplugs-dsp` for real-time-safe modulation/DSP helpers.
  - `foxplugs-plugin` for metadata and NIH-plug parameter helpers.
  - `foxplugs-ui` for reusable controls and theme primitives.
- Initial user story: “draw or choose a gain curve that repeats in time, so I can create sidechain ducking, pumping, rhythmic gating, tremolo, and transient trimming.”

## Manual-derived feature map

ShaperBox organizes one or more “Shapers” in a left-to-right processing chain. Each shaper is controlled by an editable LFO wave. VolumeShaper maps the LFO to gain: bottom is silence, middle is roughly -6 dB, and top is 0 dB/no attenuation. Its additional output Trim range is +/-24 dB. ShaperBox also includes quick presets, wave presets, wave palettes, sample-accurate oscilloscope display, multiple LFO trigger modes, MIDI/audio retriggering, multiband processing, and a global master mix/bypass.

Foxshaper should reach these capabilities in phases, starting with the minimum useful volume-shaping workflow.

## Phase 0 — Scaffold status

Already implemented in the scaffold commit:

- New `foxshaper` workspace crate.
- CLAP, VST3, and standalone export wiring.
- Basic stereo processor with a periodic unipolar triangle-shaped gain envelope.
- Parameters: `rate_hz`, `depth`, `shape`, `phase_offset`, `mix`, `output_gain`.
- Simple Vizia editor using shared Foxplugs knobs.
- Headless processor tests and no-default-feature validation.

## Phase 1 — Basic volume shaping MVP

Goal: a usable volume shaper for sidechain-style ducking and tremolo without custom drawing yet.

### DSP

- Keep zero-latency processing for the default free-running/sync mode.
- Implement a gain-mapping helper for Volume mode:
  - normalized curve value `1.0` => 0 dB/no attenuation;
  - normalized curve value `0.5` => about -6 dB;
  - normalized curve value `0.0` => silence;
  - use a smooth mapping that avoids discontinuities and clamps safely.
- Replace the current triangle-only behavior with selectable built-in shapes:
  - sidechain duck,
  - saw/ramp up,
  - saw/ramp down,
  - square/gate,
  - sine/tremolo,
  - triangle.
- Preserve current dry/wet `mix` behavior as an effect amount for volume shaping.
- Add `trim` parameter with +/-24 dB range after shaping but before output gain, or consolidate with output gain if we want only one gain stage in the MVP.
- Add smoothing/declicking at gain discontinuities, especially square/gate shapes.

### Parameters

Stable parameter IDs for Phase 1:

- `rate_hz` or `loop_rate_hz`: free-running rate.
- `depth`: amount of gain modulation.
- `shape_preset`: enum for built-in wave shapes.
- `shape`: skew/curve amount for simple shapes.
- `phase_offset`: timing offset.
- `smooth`: gain smoothing/declick amount.
- `mix`: dry/wet/effect amount.
- `trim`: output trim, +/-24 dB.
- `output_gain`: final utility gain if retained separately.

### UI

- Show one clear “Volume” page.
- Add a lightweight waveform preview for the selected shape.
- Keep knobs for rate, depth, shape/skew, phase, smooth, mix, and trim/output.
- Add preset buttons for the MVP shapes: Sidechain, Gate, Tremolo, Ramp Up, Ramp Down.

### Tests

- Shape evaluation tests at known phase positions.
- Gain mapping tests for 0%, 50%, and 100% curve values.
- Processor tests for silence, dry mix, full wet attenuation, trim/output gain, phase reset, and no extra-channel mutation.
- `cargo test -p foxshaper --lib --no-default-features` must pass.

## Phase 2 — Host-synced LFO and musical timing

Goal: make the shaper musically useful in DAW sessions.

- Add Bars/Beats mode using host transport tempo and position.
- Add loop lengths from fast rhythmic divisions through multi-bar lengths.
- Support straight, triplet, and dotted timing.
- Keep Hertz mode for free-running modulation.
- Add transport-aware phase calculation so playback from the same DAW position is deterministic.
- Add one-shot mode groundwork, even if MIDI/audio triggering lands later.

## Phase 3 — Editable curve engine

Goal: replace fixed shape presets with user-editable modulation curves.

- Introduce a real-time-safe curve representation: fixed-capacity points or precomputed lookup table swapped atomically from the UI thread.
- Point types: hard, medium, soft.
- Basic editor operations:
  - add/move/delete points;
  - snap to grid;
  - select all;
  - horizontal/vertical flip;
  - copy/paste curve inside Foxshaper;
  - undo/redo on the UI side.
- Curve tools after the basics:
  - line pen;
  - arc pen;
  - S-curve pen;
  - randomize and double-time/triple-time transforms.
- Add modulation trace display once `smooth` meaningfully changes the final modulation from the raw curve.

## Phase 4 — MIDI triggering and wave switching

Goal: performance-oriented retriggering and pattern changes.

- MIDI trigger mode: incoming notes restart the LFO.
- Optional one-shot mode: curve plays once then holds at an end marker.
- MIDI wave switching: store up to 9 wave slots and switch them from MIDI notes.
- Add channel filtering later if NIH-plug MIDI routing makes it straightforward.

## Phase 5 — Audio triggering and sidechain-aware volume shaping

Goal: sidechain-style behavior without requiring the user to draw automation manually.

- Add optional sidechain input layout.
- Detect transients in main or sidechain input.
- Threshold, detector filter range, detail/sensitivity, and trigger shift controls.
- Keep this phase opt-in because lookahead/declicking may introduce plugin latency.
- UI overlay: show external sidechain waveform over the main waveform when enabled.

## Phase 6 — Multiband volume shaping

Goal: bass-only ducking and frequency-specific rhythmic shaping.

- Split into up to three bands with adjustable crossover frequencies.
- Initial crossover slopes: pick one stable slope first; expose 6/12/24 dB per octave later.
- Per-band curve, depth, mix, and trim.
- Band solo and bypass.
- LFO link/unlink across bands.
- Validate phase response and dry/wet behavior carefully to avoid multiband combing artifacts.

## Phase 7 — Shaper rack and additional shapers

Goal: move from VolumeShaper clone toward a broader ShaperBox-style rack.

Candidate lanes after volume shaping is solid:

- Pan shaper.
- Width shaper.
- Filter shaper.
- Drive/crush shapers, reusing or coordinating with Foxcrush DSP.
- Compressor/dynamics tool.
- Standalone oscilloscope tool.

Rack features:

- Add/remove shaper modules.
- Left-to-right module ordering.
- Per-module bypass.
- Master mix and smooth master bypass.
- Preset format that stores the module chain, curves, bands, and wave slots.

## Non-goals for early phases

- Cloud preset sync.
- Exact Cableguys preset library or artwork.
- Exact proprietary transient detection, filtering, or smoothing algorithms.
- Advanced shapers before the volume-shaping workflow is genuinely useful.
- Multiband processing before single-band curve evaluation and UI editing are stable.

## Immediate next implementation tasks

1. Add `trim` and `smooth` parameters.
2. Implement a reusable volume-shaper gain mapping in `foxplugs-dsp` or local Foxshaper DSP.
3. Add fixed shape presets and replace the current triangle-only processor path.
4. Add shape/gain unit tests.
5. Add a simple waveform preview to the Foxshaper editor.
