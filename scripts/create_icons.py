#!/usr/bin/env python3
"""Generate the brand-neutral transfer mark as native bundle icons."""

from pathlib import Path
import struct
import zlib


PROVENANCE = (
    "Origin: generated locally by scripts/create_icons.py, png(256). "
    "Brand-neutral bidirectional transfer mark; warm-paper arrows on an evergreen square. "
    "Code-derived asset, no image model or external raster source."
)


def png(size: int) -> bytes:
    rows = bytearray()
    for y in range(size):
        rows.append(0)
        for x in range(size):
            a, b = x / size, y / size
            up = (0.27 < a < 0.36 and 0.30 < b < 0.73) or (0.18 < b < 0.40 and abs(a - 0.315) < (b - 0.18) * 0.8)
            down = (0.64 < a < 0.73 and 0.27 < b < 0.70) or (0.60 < b < 0.82 and abs(a - 0.685) < (0.82 - b) * 0.8)
            rows.extend((246, 244, 239, 255) if up or down else (32, 60, 51, 255))
    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xffffffff)
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
            + chunk(b"tEXt", b"impeccable:prompt\x00" + PROVENANCE.encode("ascii"))
            + chunk(b"IDAT", zlib.compress(bytes(rows), 9)) + chunk(b"IEND", b""))


def main() -> None:
    directory = Path(__file__).resolve().parents[1] / "apps/desktop/src-tauri/icons"
    directory.mkdir(parents=True, exist_ok=True)
    image = png(256)
    (directory / "icon.png").write_bytes(image)
    (directory / "icon.ico").write_bytes(struct.pack("<HHH", 0, 1, 1) + struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(image), 22) + image)
    icns = b"ic08" + struct.pack(">I", len(image) + 8) + image
    (directory / "icon.icns").write_bytes(b"icns" + struct.pack(">I", len(icns) + 8) + icns)
    print("Native transfer-mark icons generated.")


if __name__ == "__main__":
    main()
