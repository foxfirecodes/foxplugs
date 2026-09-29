# Pinned GUI lifecycle backports

These are source snapshots (including upstream MIT licenses), not a migration to
nice-plug. Cargo uses these checked-in paths. The NIH-plug revision remains
`25d5e0a0ee4b76d4c8ccd5da2cf4cb200d56e8ab` in the lockfile. Build with `--locked`.
Plugin DSP, editor layout, parameter IDs, VST3/CLAP identities and persisted state
schemas are unchanged.

| Directory | Source | Base revision |
| --- | --- | --- |
| vizia-plug | https://github.com/vizia/vizia-plug | d106863cecb020b492acf00254fb5a61d4cda55d |
| vizia | https://github.com/vizia/vizia | e91b36f2ce213eb8eefdffbb6dbc462523f2c02e |
| baseview | https://github.com/vizia/baseview | 70e4e05ba1de837970b5b5457a99893aae0b6cfa |

The snapshots were obtained with `git archive` of the exact revisions. Upstream
files not mentioned below are unchanged. `tools/vendor-diff.py` verifies that
claim and emits reviewable patches against an existing Cargo git cache.

## Runtime and attachment lifetime

Based on Vizia commit
[bbf98e643333d2533983f5866296036c34284018](https://github.com/vizia/vizia/commit/bbf98e643333d2533983f5866296036c34284018):

- Register UI threads before constructing a Context; keep the unsynchronized
  runtime assertion. Registration is counted per live window, so closing one of
  two windows sharing a native UI thread does not unregister the other.
- Baseview owns a reactive scope and registration guard on its UI thread. Context
  construction, events and frames enter that scope. The guard disposes it after
  the Context and idle captures drop, then unregisters the thread. Callbacks do
  not repeatedly register, or silently accept migration of thread-local data.
- Winit unregisters its existing registration on exit.
- Internal reactive IDs record their creating thread. Queued sync effects and
  disposals drain only on their owner; a different editor cannot consume and
  silently discard them. UI-thread writes route foreign subscribers too.
- Each Baseview window owns its own event proxy queue. Entity IDs are local to
  a Context, so a process-global queue could dispatch to the wrong editor.

Files: `vizia/crates/vizia_reactive/src/{runtime,id,state,sync_runtime}.rs`,
`vizia/crates/vizia_baseview/{Cargo.toml,src/{window,application,proxy}.rs}`,
`vizia/crates/vizia_winit/src/application.rs`.

## Parameter signals

Inspected vizia-plug commit
[281f47bc0a18f80e7f70fe8e3d5a90f274a5a0e9](https://github.com/vizia/vizia-plug/commit/281f47bc0a18f80e7f70fe8e3d5a90f274a5a0e9).
Its dependency changes target nice-plug; they are not copied. Its scope disposal
runs from the host-side handle destructor, which is not the owning Baseview
thread on Linux. This backport instead uses the backend-owned scope above and
creates a fresh ParamRegistry for every attachment. No stale signal cache is
shared between old/new attachments. Host parameter notifications only set the
existing dirty flag; each editor flushes its own signals during UI idle. Scope
cleanup and old registry destruction cannot clear a newer attachment's registry.

Files: `vizia-plug/src/{editor,lib}.rs`.

## Native redraw invalidation

Baseview already selects X11 exposure and structure events, but discarded
Expose/MapNotify. It now coalesces those notifications for its child window into
`WindowEvent::RedrawRequested`. Vizia refreshes the root and presents a new frame
even when no parameter/layout change made it dirty. No reopen resize, host hack,
or continuous repaint is added. The upstream initial-creation scaling workaround
is unchanged.

Files: `baseview/src/{event.rs,x11/{window,event_loop}.rs}` and Vizia application
handling above.

Verification and environment limits: [editor-lifecycle report](../evidence/editor-lifecycle/README.md).
