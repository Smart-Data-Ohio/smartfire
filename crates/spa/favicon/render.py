#!/usr/bin/env python3
import shutil
import struct
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path

HERE = Path(__file__).resolve().parent
SOURCE = HERE / "favicon.svg"
OFFLINE_COPY = HERE.parents[2] / "frontend/src/features/offline/favicon.svg"
SVG = "http://www.w3.org/2000/svg"


def png(svg: bytes, size: int) -> bytes:
    return subprocess.run(
        ["rsvg-convert", "--width", str(size), "--height", str(size)],
        input=svg,
        capture_output=True,
        check=True,
    ).stdout


def full_bleed(svg: bytes) -> bytes:
    """iOS rounds an apple-touch-icon's corners itself."""
    ET.register_namespace("", SVG)
    root = ET.fromstring(svg)
    del root.find(f"{{{SVG}}}rect[@id='tile']").attrib["rx"]
    return ET.tostring(root)


def ico(images: list[tuple[int, bytes]]) -> bytes:
    """An ICO holding each (size, PNG) as is, which every current browser reads."""
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = len(header) + 16 * len(images)
    entries = data = b""
    for size, image in images:
        entries += struct.pack("<BBBBHHII", size, size, 0, 0, 1, 32, len(image), offset + len(data))
        data += image
    return header + entries + data


def main() -> None:
    svg = SOURCE.read_bytes()
    (HERE / "favicon.ico").write_bytes(ico([(size, png(svg, size)) for size in (16, 32, 48)]))
    (HERE / "favicon.png").write_bytes(png(svg, 32))
    (HERE / "apple-touch-icon.png").write_bytes(png(full_bleed(svg), 180))
    shutil.copyfile(SOURCE, OFFLINE_COPY)


if __name__ == "__main__":
    main()
