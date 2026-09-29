# Isolated native editor checks

`build.py /path/to/foxdaw` builds a small eframe host against foxdaw's native VST3
host crate, without editing that checkout. Dependencies and generated manifest
live under this repository's ignored `target/`. The driver asserts exact EGL/GLX
binding preservation around editor pumping and transitions. It performs no audio
playback. A normal `quit` retires all graphs, checks retained count zero, returns
from eframe, and exits normally (no `_exit` shortcut).

Use an **owned** Xvfb display with a WM, never the user's desktop. Start Xvfb with
`-displayfd`, record its PID/display, and set software GL. The runner below masks
the home path with disposable state, mounts the checkout read-only, creates
private PID/network/session-bus namespaces and exposes no physical audio devices.

```sh
python3 tools/editor-check/build.py /path/to/foxdaw
printf 'open 0\n' > /tmp/owned-editor-commands
python3 tools/native-editor-check.py --display :OWNED --output /tmp/owned-editor-result --timeout 300 -- \
  target/gui-check/debug/foxplugs-editor-check \
  target/bundled/foxcrush.vst3/Contents/x86_64-linux/foxcrush.so \
  /tmp/owned-editor-commands
```

From another shell, append commands to that file, one per line:

- `open 0` / `open 1`: attached editor or native-parent remap.
- `close 0` / `close 1`: hide the parent while retaining the attached view. Hold
  this state at least three seconds before reopening; check `IsUnviewable` with
  `xwininfo` while closed. No diagnostic child resizing is used.
- `snapshot LABEL`: save both component states and normalized parameter values.
- `display INDEX PARAM_ID NORMALIZED`: controller-side host parameter notification.
- `recreate INDEX`: checkpoint, retire/destroy the graph and editor, load a new
  instance with that state and open its editor.
- `quit`: capture final state, retire both graphs, and leave the host normally.

An optional third driver argument is a captured snapshot JSON, used to restore
both instances in a fresh host process. Drive real controls with xdotool and
inspect screenshots, not just exit status. Foxshaper's mandatory sidechain bus
is not supported by this foxdaw host; use an isolated sidechain-capable host for
its GUI regression. The recorded REAPER run uses only rebuilt bundles and a
dummy audio engine. Its actual loaded module path is recorded in the evidence.

See [the results](../../evidence/editor-lifecycle/README.md). `verify-evidence.py`
checks the saved states and host exit receipts; screenshot inspection is separate.
