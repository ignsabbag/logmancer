#!/usr/bin/env python3
"""Headless checks for an installed suite; run as a normal user, not root."""

import fcntl
import os
from pathlib import Path
import pty
import select
import signal
import socket
import struct
import subprocess
import termios
import tempfile
import time
import urllib.error
import urllib.request


BINARIES = ("logmancer", "logmancer-desktop", "logmancer-web", "logmancer-tui")


def check_layout() -> None:
    root = Path("/usr/lib/logmancer")
    for binary in BINARIES:
        public = Path("/usr/bin") / binary
        assert public.resolve() == root / binary, public
        assert os.access(public, os.X_OK), public
        dependencies = subprocess.check_output(["ldd", str(public)], text=True)
        assert "not found" not in dependencies, dependencies
    assert Path("/usr/lib/Logmancer/site").resolve() == root / "site"
    for extension in ("css", "js", "wasm"):
        assert (root / "site/pkg" / f"logmancer-web.{extension}").stat().st_size > 0
    for size in (32, 128, 256):
        assert Path(f"/usr/share/icons/hicolor/{size}x{size}/apps/logmancer.png").is_file()
    assert Path("/usr/share/licenses/logmancer/LICENSE").is_file()
    subprocess.run(["desktop-file-validate", "/usr/share/applications/logmancer.desktop"], check=True)


def check_web(command: list[str], port: int, cwd: Path, env: dict[str, str]) -> None:
    with tempfile.TemporaryFile() as output:
        process = subprocess.Popen(command, cwd=cwd, env=env, stdout=output, stderr=output)
        try:
            deadline = time.monotonic() + 30
            url = f"http://127.0.0.1:{port}"
            while True:
                if process.poll() is not None:
                    output.seek(0)
                    raise AssertionError(output.read().decode(errors="replace"))
                try:
                    with urllib.request.urlopen(url, timeout=1) as response:
                        assert b"<html" in response.read().lower()
                    break
                except (urllib.error.URLError, TimeoutError):
                    if time.monotonic() > deadline:
                        raise AssertionError(f"Web did not start: {command}")
                    time.sleep(0.1)
            for extension in ("css", "js", "wasm"):
                with urllib.request.urlopen(f"{url}/pkg/logmancer-web.{extension}", timeout=5) as response:
                    assert response.status == 200
                    assert response.read()
            # Inspect Linux's listening sockets: no wildcard/public listeners.
            for table in ("/proc/net/tcp", "/proc/net/tcp6"):
                for line in Path(table).read_text().splitlines()[1:]:
                    fields = line.split()
                    address, listener_port = fields[1].split(":")
                    if fields[3] == "0A" and int(listener_port, 16) == port:
                        assert address == "0100007F", line
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


def check_tui(command: list[str], cwd: Path, env: dict[str, str]) -> None:
    pid, terminal = pty.fork()
    if pid == 0:
        fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 100, 0, 0))
        os.chdir(cwd)
        os.execvpe(command[0], command, env)
    reaped = False
    try:
        output = b""
        deadline = time.monotonic() + 20
        sent_quit = False
        while time.monotonic() < deadline:
            finished, status = os.waitpid(pid, os.WNOHANG)
            if finished:
                reaped = True
                assert sent_quit and os.waitstatus_to_exitcode(status) == 0, output
                return
            if select.select([terminal], [], [], 0.1)[0]:
                try:
                    output += os.read(terminal, 65536)
                except OSError:
                    continue
                if b"arch packaging smoke test" in output and not sent_quit:
                    os.write(terminal, b"q")
                    sent_quit = True
        raise AssertionError(f"TUI failed to display the file and exit: {command}: {output!r}")
    finally:
        if not reaped:
            os.kill(pid, signal.SIGKILL)
            os.waitpid(pid, 0)
        os.close(terminal)


def main() -> None:
    assert os.geteuid() != 0, "Run installed application checks as a normal user"
    check_layout()
    with tempfile.TemporaryDirectory(prefix="logmancer-arch-") as directory:
        cwd = Path(directory)
        env = {key: value for key, value in os.environ.items()
               if not key.startswith(("LEPTOS_", "LOGMANCER_", "XDG_"))
               and key not in ("DISPLAY", "WAYLAND_DISPLAY")}
        env.update(HOME=directory, TERM="xterm-256color")
        check_web(["/usr/bin/logmancer-web"], 3000, cwd, env)
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        check_web(["/usr/bin/logmancer", "web", "--bind", f"127.0.0.1:{port}"], port, cwd, env)
        log = cwd / "sample.log"
        log.write_text("arch packaging smoke test\n")
        for command in (["/usr/bin/logmancer-tui", str(log)],
                        ["/usr/bin/logmancer", "tui", str(log)],
                        ["/usr/bin/logmancer", str(log)]):
            check_tui(command, cwd, env)
        failed = subprocess.run(["/usr/bin/logmancer"], cwd=cwd, env=env, capture_output=True)
        assert failed.returncode != 0, "Bare launcher must fail without a display"
    print("Installed layout, dependencies, Web assets, loopback defaults, and TUI checks passed.")


if __name__ == "__main__":
    main()
