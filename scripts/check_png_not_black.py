#!/usr/bin/env python3
"""Fail when an 8-bit non-interlaced PNG has no non-black RGB pixel."""

import argparse
from collections import Counter
import struct
import sys
import zlib
from pathlib import Path


def paeth(left, up, up_left):
    estimate = left + up - up_left
    distances = (abs(estimate - left), abs(estimate - up), abs(estimate - up_left))
    return (left, up, up_left)[distances.index(min(distances))]


def has_visible_content(path):
    data = Path(path).read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")

    width = height = color_type = None
    compressed = bytearray()
    offset = 8
    while offset < len(data):
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        kind = data[offset + 4 : offset + 8]
        chunk = data[offset + 8 : offset + 8 + length]
        offset += length + 12
        if kind == b"IHDR":
            width, height, bit_depth, color_type, compression, filtering, interlace = struct.unpack(
                ">IIBBBBB", chunk
            )
            if (bit_depth, compression, filtering, interlace) != (8, 0, 0, 0):
                raise ValueError("only 8-bit, non-interlaced PNGs are supported")
        elif kind == b"IDAT":
            compressed.extend(chunk)
        elif kind == b"IEND":
            break

    channels = {0: 1, 2: 3, 4: 2, 6: 4}.get(color_type)
    if width is None or channels is None:
        raise ValueError("unsupported PNG color type")
    raw = zlib.decompress(compressed)
    stride = width * channels
    previous = bytearray(stride)
    colors = Counter()
    cursor = 0
    for y in range(height):
        filter_type = raw[cursor]
        cursor += 1
        row = bytearray(raw[cursor : cursor + stride])
        cursor += stride
        for index, value in enumerate(row):
            left = row[index - channels] if index >= channels else 0
            up = previous[index]
            up_left = previous[index - channels] if index >= channels else 0
            if filter_type == 1:
                row[index] = (value + left) & 0xFF
            elif filter_type == 2:
                row[index] = (value + up) & 0xFF
            elif filter_type == 3:
                row[index] = (value + ((left + up) // 2)) & 0xFF
            elif filter_type == 4:
                row[index] = (value + paeth(left, up, up_left)) & 0xFF
            elif filter_type != 0:
                raise ValueError("unsupported PNG filter")
        if 2 <= y < height - 2:
            for x in range(2, width - 2):
                pixel = row[x * channels : (x + 1) * channels]
                colors[tuple(pixel[: 1 if color_type in (0, 4) else 3])] += 1
        previous = row
    total = sum(colors.values())
    if not total:
        raise ValueError("PNG has no interior pixels")
    dominant = colors.most_common(1)[0][1]
    return total - dominant >= max(100, total // 100)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("png")
    args = parser.parse_args()
    if not has_visible_content(args.png):
        print(f"PNG has no visible interior UI contrast: {args.png}", file=sys.stderr)
        return 1
    print(f"PNG contains visible interior UI contrast: {args.png}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, zlib.error) as error:
        print(f"Cannot inspect PNG: {error}", file=sys.stderr)
        sys.exit(2)
