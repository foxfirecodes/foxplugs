# Foxcrush editor lifecycle fixes

2026-09-29. Both reported failures are repaired in the plugin dependencies. The
checkout was clean before editing, on `fix-foxcrush-editor-redraw-multi-instance`,
HEAD `9da9010ebb4a40f20362f88fd0f357fdd103f012`. The lockfile matched the supplied
vizia-plug `d106863`, Vizia `e91b36f`, and Baseview `70e4e05` revisions.

The fixes are checked-in backports on those exact versions. See
[provenance and rationale](../../vendor/README.md) and the
[upstream-relative diff](backport.diff). Vizia registers/counts each editor's UI
thread before Context construction, disposes the editor scope on that thread,
and unregisters after teardown. Parameter registries are fresh per attachment;
notifications are flushed by their own UI idle callback. Reactive work and
Baseview proxy events are routed to the owning editor. The unsynchronized-runtime
assertion remains active. X11 expose/map events now invalidate Vizia and present
a fresh frame; no reopen resize workaround was added.

DSP, parameter IDs, plugin identities, saved-state schemas, and editor layout
source are unchanged. NIH-plug remains at its original locked revision; this does
not migrate to nice-plug.

## Built artifacts

Relative to this repository:

- `target/bundled/foxcrush.vst3/Contents/x86_64-linux/foxcrush.so`
- `target/bundled/foxcrush.vst3.tar.gz` — portable copy of the complete VST3 bundle.
- `target/bundled/foxshaper.vst3/Contents/x86_64-linux/foxshaper.so`

Release VST3, CLAP and standalone bundles were built with Rust 1.93. Artifact
SHA-256s and test identities are recorded in [identity.json](identity.json).
The final Foxcrush test explicitly loads the packaged `.so` above. REAPER's
[recorded module mapping](reaper/plugin-maps.txt) points to the rebuilt Foxshaper
bundle. Nothing was installed. The installed Foxcrush still hashes to
`51a37abf06d53a885861d3320d1a43861408d6cec5704ec265cd14d74b212c08`, matching the
original diagnosis.

## Automated checks

| Check | Result |
| --- | --- |
| Workspace tests, including both DSP suites | 46 passed, zero failed. [Log](checks/workspace-tests.log). |
| Reactive runtime tests | 7 unit + 1 integration + 1 doctest passed. The integration tests separate UI threads, counted same-thread registration, rejection of non-UI access, queued host updates, writes from another UI thread, scope disposal, and recreation. [Log](checks/reactive-tests.log). |
| Vizia Baseview library target | Compiles/tests successfully; zero Linux unit tests in this upstream target. [Log](checks/baseview-tests.log). |
| Persisted GUI evidence | Independent values, stable parameter IDs, full recreation and fresh-process recall checked from actual serialized states. Two REAPER component chunks decoded and compared. [Results](state-checks.json), [script](../../tools/editor-check/verify-evidence.py). |
| Formatting and diff whitespace | Passed. [Formatting](checks/fmt-check.log). |
| Release bundles | Passed with `--locked --offline`, using a locally downloaded pinned Skia binary archive. [Foxcrush](checks/bundle-foxcrush.log), [Foxshaper](checks/bundle-foxshaper.log). |
| Strict workspace Clippy | Fails on three existing lints in unchanged `crates/foxplugs-ui/src/widgets/knob.rs`: `enum_variant_names`, `new_ret_no_self`, `too_many_arguments`. Upstream vizia-plug also emits existing compiler warnings. [Log](checks/workspace-clippy.log). |

The earliest driver queried normalized controller values before flushing queued
GUI edits. Its component-state captures were already current. The evidence
checker uses post-flush/recreation captures for normalized values, and the driver
now flushes before either query. Early screenshot/setup attempts are retained;
the successful runs below are identified explicitly.

## Visually verified native GUI behavior

All screenshots below were opened and inspected, not inferred from process exit.
The tests ran on an owned Xvfb display with xfwm4 and software GL. Native test
processes had disposable home/project state, private PID/network/session-bus
namespaces, a read-only checkout, and no physical audio devices. The running DAW,
user songs, foxdaw source, and installed plugins were not modified.

| Workflow | Observed result |
| --- | --- |
| Foxcrush first open | Complete 340×360 editor renders. [Screenshot](foxcrush-first.png). |
| Held close/reopen | The attached child stays 340×360 and becomes `IsUnviewable` while its parent is hidden. After a held close, it renders again without resizing. [Window state](foxcrush-hidden-window.txt), [reopen](foxcrush-reopened.png). Repeated cycles include explicit three-second holds. |
| Open second after closing first | Second editor opens at normal size and renders; no thread assertion or abort. [Screenshot](foxcrush-second-after-close.png). |
| Two editors simultaneously | Both render, including opening the second while the first remains open in a fresh process. [Final packaged artifact](foxcrush-final-two.png). |
| Independent controls and state | Actual knob drags produce first-instance Bit Depth 10 and second-instance Downsample 13×; other instance values remain independent. Component captures retain distinct values. [Intermediate independent edits](foxcrush-first-edited.png), [final](foxcrush-final-two.png), [states](state-checks.json). |
| Teardown/recreation and recall | Explicit graph/editor retirement and recreation preserves both instances' values. A fresh host process restores both snapshots. [Recreated editor](foxcrush-recreated.png), [fresh-process run](foxcrush-recall/result.json), [final bundle run](foxcrush-final/result.json). |
| Clean foxdaw-harness shutdown | All three completed runs exit 0, retained graph count is zero, eframe returns and the process exits normally, including exit destructors. [Final log](foxcrush-final/run.log). GLX/EGL binding assertions pass. |
| Foxshaper shared-dependency regression | Two VST3 instances on one REAPER track render simultaneously. First Depth is changed to 35%; second Shape to 79%, with its Depth still 100%. [Screenshot](foxshaper-independent.png). REAPER uses its dummy audio engine. |
| Same-plugin editor detach/recreation | Closing Foxshaper's first floating editor destroys its native child while the second remains alive. Reopening recreates a rendered editor retaining Depth 35%. [Closed tree](foxshaper-first-closed-tree.txt), [reopened tree](foxshaper-reopened-tree.txt), [screenshot](foxshaper-reopened.png). |
| Final Foxshaper bundle recall | A fresh isolated REAPER process loads the final rebuilt module and recalls both distinct states. [Screenshot](foxshaper-final-recall.png), [module mapping](reaper-final/plugin-maps.txt), [exit receipt](reaper-final/result.json). |
| REAPER save/shutdown | The disposable project contains both distinct component states; REAPER exits 0 normally. [Project](reaper/artifacts/editor-regression.rpp), [receipt](reaper/result.json). |

Foxshaper could not enter the foxdaw harness because that host rejects its
additional mandatory sidechain bus. The [failed attempt](foxshaper/run.log) is
preserved; its GUI regression was run in isolated REAPER instead. No host bus
policy or foxdaw code was changed.

## Reproduction and remaining limits

See [the test-driver instructions](../../tools/editor-check/README.md). Build:

```sh
cargo xtask bundle foxcrush --release --locked
cargo xtask bundle foxshaper --release --locked
cargo test --workspace --locked
cargo test -p vizia_reactive --locked
```

For offline Skia setup, this run downloaded the binary archive selected by
skia-bindings 0.97.0 and supplied `SKIA_BINARIES_URL=file:///absolute/archive.tar.gz`.
This is only a build-cache input, not part of the lifecycle fix.

Not verified: native Wayland/Xwayland, hardware GPU behavior, Windows/macOS,
CLAP/standalone GUI hosting, physical audio/DSP quality, or the full foxdaw mixer
workflow. Foxcrush was tested through foxdaw's real native VST3 host with two
independent graphs and real native editors; Foxshaper was tested in REAPER.
There was no sanitizer/long-duration leak run. DSP regression tests passed, but
no new audio-quality claim is made.
