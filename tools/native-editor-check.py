#!/usr/bin/env python3
"""Adapted from foxdaw tools/native-check.py (read-only test dependency).

Bounded native diagnostic in a disposable, process-local home namespace.

The enclosing checkout is read-only to the plugin. HOME is not changed; bwrap
masks its filesystem path, including libraries/config that ignore XDG variables.
The caller must supply an owned X11 display or select headless processing;
never pass a user's live desktop.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import tempfile

parser = argparse.ArgumentParser()
display_mode = parser.add_mutually_exclusive_group(required=True)
display_mode.add_argument("--display", help="an owned X11 display")
display_mode.add_argument("--headless", action="store_true",
                          help="run without X11 or Wayland display environment")
parser.add_argument("--wayland-socket", type=Path,
                    help="absolute socket of an owned compositor; X11 display remains for native editors")
parser.add_argument("--gpu-render-node", type=Path,
                    help="opt in to one DRM render node, without exposing display/modesetting devices")
parser.add_argument("--output", type=Path, required=True)
parser.add_argument("--library", type=Path)
parser.add_argument("--project-fixture", type=Path,
                    help="copy a project into the disposable HOME/.artifacts/project")
parser.add_argument("--timeout", type=int, default=30)
parser.add_argument("command", nargs=argparse.REMAINDER)
args = parser.parse_args()
if args.headless and args.wayland_socket:
    parser.error("--headless cannot be combined with --wayland-socket")
if args.wayland_socket and (not args.wayland_socket.is_absolute() or not args.wayland_socket.is_socket()):
    parser.error("--wayland-socket must be an existing absolute owned socket")
if args.gpu_render_node:
    args.gpu_render_node = args.gpu_render_node.resolve()
    if (args.gpu_render_node.parent != Path('/dev/dri')
            or not args.gpu_render_node.name.startswith('renderD')
            or not args.gpu_render_node.is_char_device()):
        parser.error("--gpu-render-node must identify one /dev/dri/renderD* character device")
class Interrupted(Exception):
    pass

def interrupted(signum, frame):
    raise Interrupted(f"signal {signum}")

signal.signal(signal.SIGTERM, interrupted)
repo = Path(__file__).resolve().parent.parent
args.output.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="foxdaw-native-") as scratch:
    private = Path(scratch) / "home"
    private.mkdir()
    if args.library:
        library = private / ".config/DecentSampler/Sample Libraries" / args.library.name
        shutil.copytree(args.library, library)
    for name in (".config", ".cache", ".local/share", ".runtime", ".artifacts"):
        (private / name).mkdir(parents=True, exist_ok=True)
    if args.project_fixture:
        shutil.copytree(args.project_fixture, private / ".artifacts/project")
    (private / ".runtime").chmod(0o700)
    portal_config = private / ".config/xdg-desktop-portal"
    portal_config.mkdir(parents=True, exist_ok=True)
    (portal_config / "portals.conf").write_text("[preferred]\ndefault=gtk\n")
    home = Path.home()
    bus_config = Path("/usr/share/dbus-1/session.conf").read_text()
    bus_config = bus_config.replace("unix:tmpdir=/tmp", f"unix:tmpdir={home / '.runtime'}")
    bus_config = bus_config.replace("<includedir>session.d</includedir>",
                                    "<includedir>/usr/share/dbus-1/session.d</includedir>")
    (private / ".runtime/session.conf").write_text(bus_config)
    env = dict(os.environ)
    env.pop("WAYLAND_DISPLAY", None)
    env.pop("DBUS_SESSION_BUS_ADDRESS", None)
    env.update(XDG_CURRENT_DESKTOP="GNOME", EGL_PLATFORM="x11", GDK_BACKEND="x11",
               LIBGL_ALWAYS_SOFTWARE="1", GTK_USE_PORTAL="0",
               XDG_CONFIG_HOME=str(home / ".config"),
               XDG_CACHE_HOME=str(home / ".cache"),
               XDG_DATA_HOME=str(home / ".local/share"),
               XDG_RUNTIME_DIR=str(home / ".runtime"),
               TMPDIR=str(home / ".runtime"),
               FOXDAW_NATIVE_ARTIFACTS=str(home / ".artifacts"),
               FONTCONFIG_FILE="/etc/fonts/fonts.conf")
    if args.headless:
        env.pop("DISPLAY", None)
    else:
        env["DISPLAY"] = args.display
    if args.wayland_socket:
        env.update(WAYLAND_DISPLAY=str(args.wayland_socket), GDK_BACKEND="wayland",
                   WINIT_UNIX_BACKEND="wayland")
        env.pop("EGL_PLATFORM", None)
    if args.gpu_render_node:
        env.pop("LIBGL_ALWAYS_SOFTWARE", None)
    command = args.command
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        parser.error("a command after -- is required")
    wrapped = ["bwrap", "--ro-bind", "/", "/", "--dev", "/dev", "--proc", "/proc", "--bind", str(private), str(home),
               "--ro-bind", str(repo), str(repo), "--unshare-net", "--unshare-pid", "--die-with-parent",
               "--new-session"]
    if args.gpu_render_node:
        wrapped += ["--dev-bind", str(args.gpu_render_node), str(args.gpu_render_node)]
    wrapped += ["--", "dbus-run-session", "--config-file", str(home / ".runtime/session.conf"), "--", *command]
    with (args.output / "run.log").open("wb") as log:
        child = subprocess.Popen(wrapped, env=env, stdout=log, stderr=subprocess.STDOUT,
                                 start_new_session=True)
        try:
            status = {"exit": child.wait(timeout=args.timeout), "timeout": False}
        except subprocess.TimeoutExpired:
            status = {"exit": None, "timeout": True}
        except Interrupted as error:
            status = {"exit": 143, "timeout": False, "interrupted": str(error)}
        finally:
            # The private PID namespace owns *all* test descendants, including
            # daemonized portal/plugin children. Killing its bwrap owner tears
            # down that namespace before temporary files are copied or removed.
            # An interrupted experiment is never reported as clean native close.
            signal.signal(signal.SIGTERM, signal.SIG_IGN)
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
    status.update(command=command, display=args.display, timeout_seconds=args.timeout,
                  isolation="read-only root/checkout; disposable home; private PID namespace, session bus and GTK portal; no network",
                  software_gl=args.gpu_render_node is None,
                  wayland_socket=str(args.wayland_socket) if args.wayland_socket else None,
                  gpu_render_node=str(args.gpu_render_node) if args.gpu_render_node else None)
    (args.output / "result.json").write_text(json.dumps(status, indent=2) + "\n")
    shutil.copytree(private / ".artifacts", args.output / "artifacts", dirs_exist_ok=True)
    print(json.dumps(status))
    raise SystemExit(1 if status["timeout"] else status["exit"])
