#!/usr/bin/env python3
"""Record one terminal keypress and explain how Codex can bind it."""

from __future__ import annotations

import argparse
import os
import select
import sys
import termios
import time
import tty


CTRL_NAMES = {
    0x00: "ctrl+space",
    **{i: f"ctrl+{chr(ord('a') + i - 1)}" for i in range(1, 27)},
    0x1B: "esc",
    0x1C: "ctrl+4",
    0x1D: "ctrl+5",
    0x1E: "ctrl+6",
    0x1F: "ctrl+7",
}


def escaped(data: bytes) -> str:
    out: list[str] = []
    for byte in data:
        if byte == 0x1B:
            out.append("\\x1b")
        elif byte == 0x0D:
            out.append("\\r")
        elif byte == 0x0A:
            out.append("\\n")
        elif byte == 0x09:
            out.append("\\t")
        elif 32 <= byte <= 126:
            out.append(chr(byte))
        else:
            out.append(f"\\x{byte:02x}")
    return "".join(out)


def modifier_names(csi_u_modifier: int) -> list[str]:
    # CSI-u encodes modifiers as 1 + bitset: shift=1, alt=2, ctrl=4.
    bits = csi_u_modifier - 1
    names: list[str] = []
    if bits & 4:
        names.append("ctrl")
    if bits & 1:
        names.append("shift")
    if bits & 2:
        names.append("alt")
    return names


def parse_csi_u(data: bytes) -> str | None:
    if not (data.startswith(b"\x1b[") and data.endswith(b"u")):
        return None
    body = data[2:-1].decode("ascii", "ignore")
    parts = body.split(";")
    if not parts or not parts[0].isdigit():
        return None
    codepoint = int(parts[0])
    key = chr(codepoint).lower() if 0 <= codepoint <= 0x10FFFF else f"u+{codepoint:x}"
    mods = modifier_names(int(parts[1])) if len(parts) > 1 and parts[1].isdigit() else []
    spec = "-".join([*mods, key]) if mods else key
    return f"CSI-u event: {spec}\nCodex key spec: {spec}"


def explain(data: bytes) -> str:
    lines = [
        f"bytes: {' '.join(f'0x{byte:02x}' for byte in data) or '<none>'}",
        f"text : {escaped(data) or '<none>'}",
    ]

    csi_u = parse_csi_u(data)
    if csi_u:
        lines.append(csi_u)
        return "\n".join(lines)

    if len(data) == 1 and data[0] in CTRL_NAMES:
        spec = CTRL_NAMES[data[0]]
        lines.append(f"legacy control event: {spec}")
        if data[0] == 0x04:
            lines.append("Terminal did not send Shift; ctrl+d and ctrl+shift+d are indistinguishable here.")
        lines.append(f"Codex key spec: {spec.replace('+', '-')}")
        return "\n".join(lines)

    if not data:
        lines.append("No bytes arrived. The OS or terminal consumed the shortcut before Codex could see it.")
        return "\n".join(lines)

    lines.append("Unrecognized byte sequence. Use this exact text mapping if your terminal supports sending text.")
    lines.append(f"Ghostty text action: text:{escaped(data)}")
    return "\n".join(lines)


def read_key(timeout: float) -> bytes:
    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    try:
        tty.setraw(fd)
        sys.stdout.write("press shortcut now... ")
        sys.stdout.flush()
        deadline = time.monotonic() + timeout
        data = bytearray()
        while time.monotonic() < deadline:
            ready, _, _ = select.select([fd], [], [], max(0.0, min(0.1, deadline - time.monotonic())))
            if not ready:
                if data:
                    break
                continue
            chunk = os.read(fd, 64)
            if chunk:
                data.extend(chunk)
                time.sleep(0.05)
            elif data:
                break
        return bytes(data)
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)
        sys.stdout.write("\n")
        sys.stdout.flush()


def main() -> int:
    parser = argparse.ArgumentParser(description="Probe raw terminal bytes for one shortcut.")
    parser.add_argument("--count", type=int, default=1)
    parser.add_argument("--timeout", type=float, default=5.0)
    args = parser.parse_args()

    for index in range(args.count):
        if args.count > 1:
            print(f"\nprobe {index + 1}/{args.count}")
        print(explain(read_key(args.timeout)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
