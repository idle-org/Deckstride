#!/usr/bin/env python3
"""Run GUI tests under an isolated Xvfb display on POSIX CI workers."""
import os
import select
import shutil
import subprocess
import sys
import tempfile


def run(command):
    if not command:
        raise ValueError("usage: gui_ci.py COMMAND [ARGS...]")
    xvfb = shutil.which("Xvfb")
    if xvfb is None:
        raise RuntimeError("Xvfb is required for native GUI CI/coverage; uv cannot install the X server. Provision the worker with xvfb (Debian/Ubuntu) or xorg-vfbserver (FreeBSD), and put Xvfb on PATH.")
    reader, writer = os.pipe()
    server = None
    with tempfile.TemporaryFile() as log:
        try:
            server = subprocess.Popen(
                [xvfb, "-displayfd", str(writer), "-screen", "0", "1920x1080x24", "-nolisten", "tcp"],
                pass_fds=(writer,), stdout=log, stderr=log,
            )
            os.close(writer)
            writer = None
            if not select.select([reader], [], [], 10)[0]:
                raise RuntimeError("Xvfb did not become ready within 10 seconds")
            number = os.read(reader, 64).decode("ascii").strip()
            if not number.isdecimal() or len(number) > 5:
                log.seek(0)
                raise RuntimeError("Xvfb startup failed: " + log.read(2048).decode(errors="replace"))
            env = dict(os.environ, DISPLAY=f":{number}", DECKSTRIDE_GUI_TESTS="1")
            env.pop("WAYLAND_DISPLAY", None)
            env.pop("WAYLAND_SOCKET", None)
            return subprocess.run(command, env=env, timeout=1800).returncode
        finally:
            os.close(reader)
            if writer is not None:
                os.close(writer)
            if server is not None:
                server.terminate()
                try:
                    server.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    server.kill()
                    server.wait(timeout=5)


if __name__ == "__main__":
    try:
        sys.exit(run(sys.argv[1:]))
    except (OSError, RuntimeError, ValueError, subprocess.TimeoutExpired) as error:
        print(f"GUI CI failed: {error}", file=sys.stderr)
        sys.exit(1)
