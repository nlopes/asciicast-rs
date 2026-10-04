#!/usr/bin/env python3
"""Capture original benchmark workloads through a PTY (Unix, Python 3 only)."""

import argparse
import codecs
import errno
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import struct
import subprocess
import sys
import tempfile
import termios
import time


COLS, ROWS = 100, 30
WORKLOADS = {
    "typing": "Short writes, command editing, quotes, backslashes and Unicode output",
    "logs": "Bursts of coloured build-style logs and carriage-return progress updates",
    "dashboard": "Full-screen redraws with cursor movement, colours and Unicode",
}


def emit(text):
    sys.stdout.write(text)
    sys.stdout.flush()


def workload(name):
    # All displayed content is authored here. No interactive shell, user files,
    # command history, network access, or external program output is captured.
    if name == "typing":
        for index in range(12):
            emit("\x1b[32mdemo\x1b[0m $ ")
            for char in f'print "recording {index:02d}: café 東京 🚀"':
                emit(char)
                time.sleep(0.002)
            emit(" typo")
            time.sleep(0.02)
            emit("\b \b" * 5 + "\n")
            emit(f'recording {index:02d}: café 東京 🚀 — path C:\\demo\\file\n')
            time.sleep(0.025)
        emit("demo $ exit\n")
    elif name == "logs":
        emit("Starting local demonstration build\n")
        for index in range(1024):
            colour = 33 if index % 29 == 0 else 32
            state = "warning" if index % 29 == 0 else "complete"
            emit(
                f'\x1b[{colour}m[{index:04d}] {state:8}\x1b[0m '
                f'module_{index:04d} elapsed={(index * 137) % 10000:04d}us '
                f'checksum={(index * 2654435761) % (1 << 32):08x} '
                'message="compiled successfully"\n'
            )
            if index % 16 == 0:
                emit(f"\r\x1b[2Kprogress {index * 100 // 1024:3d}%")
                time.sleep(0.004 + (index % 3) * 0.001)
                emit("\r\x1b[2K")
        emit("Build complete: 1024 modules\n")
    else:
        emit("\x1b[?1049h\x1b[?25l\x1b[2J")
        for frame in range(120):
            lines = [f"\x1b[H\x1b[1;36mLocal dashboard — frame {frame:03d} 🚀\x1b[0m\x1b[K"]
            for row in range(24):
                percent = (frame * 7 + row * 13) % 101
                filled = percent * 32 // 100
                bar = "█" * filled + "░" * (32 - filled)
                lines.append(
                    f"\x1b[{row + 2};1Hworker {row:02d} │ "
                    f"\x1b[{32 + row % 5}m{bar}\x1b[0m │ {percent:3d}% "
                    f"jobs={frame * 24 + row:05d} café 東京\x1b[K"
                )
            emit("".join(lines))
            time.sleep(0.016)
        emit("\x1b[?25h\x1b[?1049lDashboard complete\n")


def capture(name):
    master, slave = pty.openpty()
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
        decoder = codecs.getincrementaldecoder("utf-8")()
        events = []
        with tempfile.TemporaryDirectory(prefix="asciicast-capture-") as directory:
            started = time.monotonic_ns()
            with subprocess.Popen(
                [sys.executable, "-I", str(Path(__file__).resolve()), "--workload", name],
                stdin=slave,
                stdout=slave,
                stderr=slave,
                cwd=directory,
                env={"TERM": "xterm-256color", "LANG": "C.UTF-8", "PYTHONIOENCODING": "utf-8"},
                start_new_session=True,
            ) as child:
                os.close(slave)
                slave = None
                while True:
                    try:
                        chunk = os.read(master, 65536)
                    except OSError as error:
                        if error.errno != errno.EIO:  # Linux PTY end-of-file
                            raise
                        chunk = b""
                    at_us = (time.monotonic_ns() - started) // 1000
                    text = decoder.decode(chunk, final=not chunk)
                    if text:
                        events.append((at_us, text))
                    if not chunk:
                        break
                if child.wait() != 0:
                    raise RuntimeError(f"{name}: workload failed")
        return events
    finally:
        os.close(master)
        if slave is not None:
            os.close(slave)


def encode(name, events, version):
    header = {"version": version, "title": f"Local benchmark: {name}"}
    if version == 2:
        header.update(width=COLS, height=ROWS)
    else:
        header["term"] = {"cols": COLS, "rows": ROWS, "type": "xterm-256color"}
    lines = [json.dumps(header, ensure_ascii=False, separators=(",", ":"))]
    previous = 0
    for at_us, text in events:
        timing = at_us if version == 2 else at_us - previous
        # Integer microseconds avoid floating-point drift when deriving v3
        # intervals from the same capture used for the v2 absolute timestamps.
        timestamp = f"{timing // 1000000}.{timing % 1000000:06d}"
        payload = json.dumps(text, ensure_ascii=False)
        lines.append(f'[{timestamp},"o",{payload}]')
        previous = at_us
    return ("\n".join(lines) + "\n").encode("utf-8")


def record(directory):
    # Captures vary with scheduling. Preserve old fixtures/baselines and require
    # an explicit review/copy step when refreshing a checked-in corpus.
    directory.mkdir(parents=True, exist_ok=False)
    manifest = []
    for name, description in WORKLOADS.items():
        events = capture(name)
        for version in (2, 3):
            filename = f"{name}-v{version}.cast"
            data = encode(name, events, version)
            (directory / filename).write_bytes(data)
            manifest.append({
                "name": filename,
                "version": version,
                "events": len(events),
                "bytes": len(data),
                "origin": "locally-recorded",
                "license": "MIT OR Apache-2.0",
                "generator": "scripts/record-corpus.py",
                "description": description,
                "sha256": hashlib.sha256(data).hexdigest(),
            })
            print(f"Recorded {filename}: {len(events)} events, {len(data)} bytes")
    (directory / "corpus.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", nargs="?", type=Path, default=Path("target/recorded-corpus"))
    parser.add_argument("--workload", choices=WORKLOADS, help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.workload:
        workload(args.workload)
    else:
        record(args.directory)


if __name__ == "__main__":
    main()
