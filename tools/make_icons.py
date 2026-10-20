#!/usr/bin/env python3
"""Генерация иконок приложения (PNG, без внешних зависимостей).

Рисует шестигранную галерею в палитре Catppuccin Mocha и пишет PNG
собственной реализацией (zlib + struct).
"""
import math
import os
import struct
import zlib

BASE = (0x1E, 0x1E, 0x2E)
CRUST = (0x11, 0x11, 0x1B)
MAUVE = (0xCB, 0xA6, 0xF7)
TEAL = (0x94, 0xE2, 0xD5)
LAP = (0xB4, 0xBE, 0xFE)
SURFACE = (0x31, 0x32, 0x44)


def lerp(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


def blend(dst, src, alpha):
    return tuple(int(round(dst[i] * (1 - alpha) + src[i] * alpha)) for i in range(3))


def hex_dist(px, py, cx, cy, r):
    """Приблизительное расстояние до границы правильного шестиугольника."""
    # Шестиугольник с вершинами под углами 0, 60, ... в «плоской» ориентации.
    ang = math.atan2(py - cy, px - cx)
    ap = math.pi / 3.0
    seg = math.floor((ang + ap / 2) / ap)
    na = (seg * ap) - ap / 2
    ex = cx + r * math.cos(na)
    ey = cy + r * math.sin(na)
    # Расстояние до отрезка от вершины до следующей.
    a0 = na
    a1 = na + ap
    x0, y0 = cx + r * math.cos(a0), cy + r * math.sin(a0)
    x1, y1 = cx + r * math.cos(a1), cy + r * math.sin(a1)
    vx, vy = x1 - x0, y1 - y0
    wx, wy = px - x0, py - y0
    t = max(0.0, min(1.0, (wx * vx + wy * vy) / (vx * vx + vy * vy)))
    dx, dy = wx - t * vx, wy - t * vy
    return math.hypot(dx, dy)


def render(size):
    px = bytearray()
    c = size / 2.0
    r_outer = size * 0.42
    r_mid = size * 0.28
    r_inner = size * 0.15
    stroke_outer = size * 0.055
    stroke_mid = size * 0.042
    for y in range(size):
        px.append(0)  # PNG filter: None
        for x in range(size):
            fx, fy = x + 0.5, y + 0.5
            col = BASE
            # Мягкий радиальный градиент.
            d = math.hypot(fx - c, fy - c) / (size / 2.0)
            col = blend(col, CRUST, max(0.0, min(1.0, d * 0.9)))
            # Свечение вокруг гекса.
            glow = max(0.0, 1.0 - abs(hex_dist(fx, fy, c, c, r_outer) - stroke_outer) / (size * 0.22))
            col = blend(col, MAUVE, glow * 0.10)
            # Внешний контур.
            d_o = abs(hex_dist(fx, fy, c, c, r_outer) - stroke_outer)
            if d_o < stroke_outer * 0.55:
                col = blend(col, MAUVE, 1.0)
            # Средний контур.
            d_m = abs(hex_dist(fx, fy, c, c, r_mid) - stroke_mid)
            if d_m < stroke_mid * 0.5:
                col = blend(col, LAP, 0.85)
            # Внутренний заполненный гекс.
            d_i = hex_dist(fx, fy, c, c, r_inner)
            if d_i < r_inner:
                col = blend(col, TEAL, 0.85)
            elif d_i < r_inner + size * 0.02:
                col = blend(col, TEAL, 0.35)
            # Подложка-«полка» снизу.
            if fy > size * 0.86 and abs(fx - c) < size * 0.26:
                col = blend(col, SURFACE, 0.85)
            px.extend(col)
    return bytes(px)


def write_png(path, size):
    raw = render(size)
    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    # Тип 6 = RGBA. Tauri требует альфа-канал: на Windows сборка проходит и с
    # RGB (тип 2), а на Linux generate_context! падает с "icon ... is not RGBA".
    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(png)
    print(f"{path} ({len(png)} bytes)")


def write_ico(path, sizes):
    """ICO с PNG-внутри (Vista+)."""
    images = []
    for s in sizes:
        raw = render(s)
        def chunk(tag, data):
            c = struct.pack(">I", len(data)) + tag + data
            return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        header = struct.pack(">IIBBBBB", s, s, 8, 2, 0, 0, 0)
        png = (
            b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", header)
            + chunk(b"IDAT", zlib.compress(raw, 9))
            + chunk(b"IEND", b"")
        )
        images.append((s, png))
    out = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    entries = b""
    for s, png in images:
        entries += struct.pack(
            "<BBBBHHII", s if s < 256 else 0, s if s < 256 else 0, 0, 0, 1, 32, len(png), offset
        )
        offset += len(png)
    with open(path, "wb") as f:
        f.write(out + entries + b"".join(png for _, png in images))
    print(f"{path} ({len(out + entries)} bytes header)")


if __name__ == "__main__":
    project = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    root = os.path.join(project, "src-tauri", "icons")
    pub = os.path.join(project, "public", "icons")
    write_png(os.path.join(root, "32x32.png"), 32)
    write_png(os.path.join(root, "128x128.png"), 128)
    write_png(os.path.join(root, "128x128@2x.png"), 256)
    write_png(os.path.join(root, "icon.png"), 512)
    write_png(os.path.join(pub, "32x32.png"), 32)
    write_ico(os.path.join(root, "icon.ico"), [16, 32, 48, 64, 128, 256])
    # .icns для macOS — Tauri принимает PNG внутри icns.
    src = os.path.join(root, "icon.png")
    with open(src, "rb") as f:
        png = f.read()
    with open(os.path.join(root, "icon.icns"), "wb") as f:
        f.write(b"icns" + struct.pack(">I", 8 + len(png)) + b"ic09" + struct.pack(">I", 8 + len(png)) + png)
    print("icon.icns (ic09)")
