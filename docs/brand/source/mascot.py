#!/usr/bin/env python3
import pathlib
import subprocess
import sys

SOURCE = pathlib.Path(__file__).resolve().parent
BRAND = SOURCE.parent
RENDER = SOURCE / "render" / "target" / "release" / "brand-render"
FONT = SOURCE.parent.parent.parent / "crates" / "app" / "assets" / "Mitr-SemiBold.ttf"

FEATHER = "#1C1A17"
WATTLE = "#FFD23F"
BEAK = "#F28C28"
BEAK_SHADE = "#E07A16"
BEAK_SEAM = "#C96A12"
PAPER = "#FBF7EE"
LIVE = "#E5484D"
INK2 = "#6B645A"
SIDE = "#F6F1E4"
SELECT = "#FFE89A"

LIGHT = {
    "ink": FEATHER,
    "wing": "#2B2824",
    "rim": None,
    "shadow": (FEATHER, 0.12),
    "zzz": INK2,
    "mic": INK2,
    "mic_handle": FEATHER,
    "cable": INK2,
    "plug": INK2,
}
DARK = {
    "ink": PAPER,
    "wing": "#3A352E",
    "rim": SIDE,
    "shadow": ("#000000", 0.4),
    "zzz": "#B5AB99",
    "mic": "#A89F8F",
    "mic_handle": SIDE,
    "cable": "#A89F8F",
    "plug": "#A89F8F",
}

_text_cache = {}


def text_path(text, size, spacing=0):
    key = (text, size, spacing)
    if key not in _text_cache:
        out = subprocess.run(
            [str(RENDER), "textpath", str(FONT), "Mitr", str(size), str(spacing), text],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.splitlines()
        x, y, w, h = (float(v) for v in out[0].split())
        _text_cache[key] = (x, y, w, h, out[1])
    return _text_cache[key]


def glyph(text, size, cx, cy, fill, spacing=0):
    x, y, w, h, d = text_path(text, size, spacing)
    tx = cx - (x + w / 2)
    ty = cy - (y + h / 2)
    return f'<path transform="translate({tx:.2f} {ty:.2f})" fill="{fill}" d="{d}"/>'


def svg(view, body, title, extra=""):
    w, h = view
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" '
        f'role="img" aria-label="{title}"{extra}>\n{body}\n</svg>\n'
    )


class Rim:
    def __init__(self, theme):
        self.color = theme["rim"]

    def stroke(self, width):
        if not self.color:
            return ""
        return f' stroke="{self.color}" stroke-width="{width}" stroke-linejoin="round" stroke-linecap="round"'


BODY = "M28 84C26 58 42 44 62 44C84 44 96 60 94 82C92 100 78 108 60 108C42 108 30 100 28 84Z"
TAIL = "M33 82C25 86 20 96 23 106C30 104 36 97 38 88Z"
RAISED_WING = "M48 76C32 74 18 62 14 44C13 40 17 38 20 40C32 48 44 58 52 66Z"
TUFT = "M57 19C58 12 65 10 69 14"
WING = "M62 63C76 65 81 80 71 92C63 100 48 102 35 99C43 92 49 84 55 74C57 69 59 65 62 63Z"
WING_LINE = "M60 77C67 83 66 91 58 96"


def bird(theme, tilt=0, eye="open", beak="shut", wing="rest"):
    rim = Rim(theme)
    parts = []
    shadow, opacity = theme["shadow"]
    parts.append(f'<ellipse cx="60" cy="113" rx="29" ry="3.4" fill="{shadow}" opacity="{opacity}"/>')
    feet = "".join(
        f'<path d="M{x} 104V111.5M{x - 5} 115L{x} 111.5L{x + 5} 115"/>' for x in (51, 71)
    )
    parts.append(f'<g fill="none" stroke="{BEAK}" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">{feet}</g>')

    shapes = [("path", f'd="{TAIL}"'), ("path", f'd="{BODY}"')]
    if wing == "raised":
        shapes.insert(0, ("path", f'd="{RAISED_WING}"'))
    if rim.color:
        for tag, attrs in shapes:
            parts.append(f'<{tag} {attrs} fill="{rim.color}"{rim.stroke(5)}/>')
        parts.append(
            f'<g transform="rotate({tilt} 60 42)"><circle cx="60" cy="40" r="24" fill="{rim.color}"{rim.stroke(5)}/>'
            f'<path d="{TUFT}" fill="none" stroke="{rim.color}" stroke-width="9" stroke-linecap="round"/></g>'
        )
    for tag, attrs in shapes:
        parts.append(f'<{tag} {attrs} fill="{FEATHER}"/>')
    if wing == "rest":
        parts.append(f'<path d="{WING}" fill="{theme["wing"]}"/>')
        parts.append(f'<path d="{WING_LINE}" fill="none" stroke="{PAPER}" stroke-width="3" stroke-linecap="round" opacity="0.9"/>')
    else:
        parts.append(f'<path d="M23 46C29 56 37 63 46 69" fill="none" stroke="{PAPER}" stroke-width="2.6" stroke-linecap="round" opacity="0.85"/>')

    head = []
    head.append(f'<circle cx="60" cy="40" r="24" fill="{FEATHER}"/>')
    head.append(f'<path d="{TUFT}" fill="none" stroke="{FEATHER}" stroke-width="4.5" stroke-linecap="round"/>')
    head.append(f'<path d="M65 45C74 40 83 45 82 54C79 61 70 60 66 54C64 51 64 48 65 45Z" fill="{WATTLE}"/>')
    if beak == "open":
        head.append(f'<path d="M77 32C87 29 96 29 102 32C96 37 88 38.5 78 38.5Z" fill="{BEAK}"/>')
        head.append(f'<path d="M78 42C86 41.5 93 43 96.5 47C89 48 82 47 78 46Z" fill="{BEAK_SHADE}"/>')
    else:
        head.append(f'<path d="M77 34C87 32 96 33 101 38C95 42.5 86 43 77 42Z" fill="{BEAK}"/>')
        head.append(f'<path d="M80 38.2H98.5" stroke="{BEAK_SEAM}" stroke-width="1.6" stroke-linecap="round"/>')
    if eye == "closed":
        head.append(f'<path d="M62 34C65 37.5 69 37.5 72 34" fill="none" stroke="{PAPER}" stroke-width="2.6" stroke-linecap="round"/>')
    else:
        head.append(f'<circle cx="67" cy="33" r="5.2" fill="{PAPER}"/><circle cx="68.4" cy="33" r="2.8" fill="{FEATHER}"/><circle cx="69.5" cy="31.8" r="1" fill="#FFFFFF"/>')
    parts.append(f'<g transform="rotate({tilt} 60 42)">' + "".join(head) + "</g>")
    return "\n".join(parts)


def pose(name, theme):
    ink = theme["ink"]
    if name == "default":
        body = bird(theme)
    elif name == "listening":
        body = bird(theme, tilt=-12)
        body += (
            f'\n<g fill="none" stroke="{LIVE}" stroke-width="3" stroke-linecap="round">'
            '<path d="M20 32C14 39 14 49 20 56"/><path d="M11 25C2 37 2 51 11 63"/></g>'
        )
    elif name == "writing":
        body = bird(theme, beak="open")
        body += "\n" + glyph("ก", 27, 107, 19, ink) + "\n" + glyph("ข", 20, 111, 41, ink)
        body += f'\n<rect x="100" y="52" width="16" height="3" rx="1.5" fill="{WATTLE}"/>'
    elif name == "note":
        body = bird(theme, tilt=6)
        body += (
            '\n<g transform="rotate(-8 22 70)">'
            f'<rect x="6" y="54" width="30" height="36" rx="3" fill="{PAPER}" stroke="{ink}" stroke-width="2.5"/>'
            f'<path d="M12 64H30M12 71H30M12 78H24" stroke="{FEATHER}" stroke-width="2" stroke-linecap="round"/>'
            f'<rect x="14" y="50" width="14" height="7" rx="2" fill="{WATTLE}" stroke="{ink}" stroke-width="2.5"/></g>'
        )
    elif name == "confused":
        body = bird(theme, tilt=14)
        body += "\n" + glyph("?", 34, 100, 22, BEAK)
        body += (
            '\n<g transform="translate(10 62)">'
            f'<path d="M4 8V5C4 -2 16 -2 16 5V8" fill="none" stroke="{ink}" stroke-width="2.5" stroke-linecap="round"/>'
            f'<rect x="0" y="8" width="20" height="16" rx="3" fill="{WATTLE}" stroke="{ink}" stroke-width="2.5"/>'
            f'<circle cx="10" cy="15" r="2" fill="{FEATHER}"/><path d="M10 16V20" stroke="{FEATHER}" stroke-width="2" stroke-linecap="round"/></g>'
        )
    elif name == "sleeping":
        body = bird(theme, tilt=10, eye="closed")
        body += "\n" + glyph("z", 24, 91, 25, theme["zzz"]) + "\n" + glyph("z", 17, 105, 10, theme["zzz"])
    elif name == "waving":
        body = bird(theme, wing="raised")
        body += (
            f'\n<g fill="none" stroke="{BEAK}" stroke-width="2.6" stroke-linecap="round">'
            '<path d="M8 46H3"/><path d="M9 35L5 31"/><path d="M15 29L13.5 24"/></g>'
        )
    elif name == "oops":
        body = bird(theme, tilt=8)
        body += (
            '\n<g transform="translate(104 66) rotate(24)">'
            f'<rect x="-7" y="-22" width="14" height="24" rx="7" fill="{theme["mic"]}"/>'
            f'<path d="M-7 -14H7M-7 -9H7M-7 -4H7" stroke="{PAPER}" stroke-width="1.5" opacity="0.75"/>'
            f'<rect x="-4" y="2" width="8" height="11" rx="2.5" fill="{theme["mic_handle"]}"/></g>'
            f'\n<path d="M97.3 78.8C96 92 111 93 108 103" fill="none" stroke="{theme["cable"]}" stroke-width="2.6" stroke-linecap="round"/>'
            f'<rect x="103.5" y="103" width="9" height="7" rx="1.5" fill="{theme["plug"]}"/>'
            f'<path d="M106 110V114M110 110V114" stroke="{theme["plug"]}" stroke-width="2" stroke-linecap="round"/>'
            f'<path d="M116 98L119 95M117 104H120.5M113.5 94L114.5 91" stroke="{WATTLE}" stroke-width="2.2" stroke-linecap="round"/>'
        )
        body += "\n" + glyph("!", 30, 17, 24, BEAK)
    elif name == "searching":
        body = bird(theme, tilt=-4)
        body += (
            f'\n<path d="M106.5 84.5L116 95" stroke="{BEAK}" stroke-width="5.5" stroke-linecap="round"/>'
            f'<circle cx="98" cy="76" r="12" fill="{SELECT}" fill-opacity="0.55" stroke="{ink}" stroke-width="3.2"/>'
            f'<path d="M91 74C92 69.5 95.5 67.5 99 67.8" fill="none" stroke="{PAPER}" stroke-width="2.4" stroke-linecap="round"/>'
        )
    else:
        raise ValueError(name)
    return body


def head(state, theme, size=64):
    rim = Rim(theme)
    cx, cy, r = 31, 38, 21
    tuft = "M25 20C26 13 33 11 37 14"
    parts = []
    if rim.color:
        parts.append(f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="{rim.color}"{rim.stroke(6)}/>')
        parts.append(f'<path d="{tuft}" fill="none" stroke="{rim.color}" stroke-width="10" stroke-linecap="round"/>')
    parts.append(f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="{FEATHER}"/>')
    parts.append(f'<path d="{tuft}" fill="none" stroke="{FEATHER}" stroke-width="4.4" stroke-linecap="round"/>')
    parts.append(f'<path d="M34 42C41 38.5 48 42.5 47 50C45 55 38.5 54.5 36 49.5C35 47 34 45 34 42Z" fill="{WATTLE}"/>')
    if state == "writing":
        parts.append(f'<path d="M46 31C53 28 59.5 28.5 63.5 31.5C58 35.5 52 36 46 36Z" fill="{BEAK}"/>')
        parts.append(f'<path d="M46 40C52 39.5 58 41 61 45C55 46 50 45.5 46 44Z" fill="{BEAK_SHADE}"/>')
    else:
        parts.append(f'<path d="M46 33C54 31 60.5 32.5 63.5 36C58.5 40 52 40.5 46 40Z" fill="{BEAK}"/>')
        parts.append(f'<path d="M48.5 36.4H61.5" stroke="{BEAK_SEAM}" stroke-width="1.4" stroke-linecap="round"/>')
    parts.append(f'<circle cx="39" cy="31" r="5.2" fill="{PAPER}"/><circle cx="40.3" cy="31" r="2.8" fill="{FEATHER}"/><circle cx="41.3" cy="29.9" r="1" fill="#FFFFFF"/>')
    if state == "listening":
        parts.append(f'<circle cx="10" cy="11" r="7" fill="{LIVE}"/>')
    if state == "confused":
        parts.append(glyph("?", 26, 9.5, 13, BEAK))
    return "\n".join(parts)


def menubar(state):
    glyph_shapes = (
        '<circle cx="8" cy="9.6" r="6.6"/>'
        '<path d="M12.4 7.6C14.6 6.9 16.8 7.5 17.8 9C16.6 10.8 14.6 11 12.4 10.8Z"/>'
        '<path d="M5.4 4.4C5.6 2.4 7.2 1.6 8.6 2.2" fill="none" stroke="#000" stroke-width="1.5" stroke-linecap="round"/>'
    )
    cuts = [
        '<circle cx="9.7" cy="7.6" r="1.25"/>',
        '<path d="M8.4 10.9C10 10 12 10.9 11.6 12.6C11 13.7 9.2 13.4 8.7 12.3C8.4 11.8 8.3 11.4 8.4 10.9Z"/>',
    ]
    accent = ""
    defs = ""
    if state == "listening":
        cuts.append('<circle cx="3" cy="3.4" r="3.3"/>')
        accent = '<circle cx="3" cy="3.4" r="2.3"/>'
    elif state == "attention":
        cuts.append('<circle cx="14.2" cy="14.2" r="4.2"/>')
        defs = (
            '<mask id="b" maskUnits="userSpaceOnUse" x="0" y="0" width="18" height="18">'
            '<rect width="18" height="18" fill="#fff"/>'
            '<path d="M14.2 11.9V14.3M14.2 15.9V16" stroke="#000" stroke-width="1.3" stroke-linecap="round" fill="none"/></mask>'
        )
        accent = '<circle cx="14.2" cy="14.2" r="3.2" mask="url(#b)"/>'
    elif state == "transcribing":
        cuts.append('<rect x="10.4" y="13.4" width="7.4" height="4.4" rx="2.2"/>')
        accent = '<circle cx="12.1" cy="15.6" r="1"/><circle cx="14.1" cy="15.6" r="1"/><circle cx="16.1" cy="15.6" r="1"/>'
    mask = (
        '<mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="18" height="18">'
        '<rect width="18" height="18" fill="#fff"/><g fill="#000">' + "".join(cuts) + "</g></mask>"
    )
    return (
        f"<defs>{mask}{defs}</defs>\n"
        f'<g fill="#000" mask="url(#m)">{glyph_shapes}</g>\n'
        f'<g fill="#000">{accent}</g>'
    )


def icon(maskable=False):
    s = 9.0 if maskable else 11.0
    tx = 512 - 36 * s
    ty = 512 - 35 * s
    theme = LIGHT
    h = head("default", theme).replace(WATTLE, WATTLE)
    parts = ['<defs>']
    if not maskable:
        parts.append(
            '<clipPath id="tile"><rect x="100" y="100" width="824" height="824" rx="185"/></clipPath>'
            '<filter id="lift" x="-10%" y="-10%" width="120%" height="125%"><feDropShadow dx="0" dy="14" stdDeviation="16" flood-color="#3C2800" flood-opacity="0.28"/></filter>'
        )
    else:
        parts.append('<clipPath id="tile"><rect width="1024" height="1024"/></clipPath>')
    parts.append("</defs>")
    if maskable:
        parts.append(f'<rect width="1024" height="1024" fill="{WATTLE}"/>')
    else:
        parts.append(f'<rect x="100" y="100" width="824" height="824" rx="185" fill="{WATTLE}" filter="url(#lift)"/>')
    parts.append('<g clip-path="url(#tile)">')
    parts.append('<path d="M0 760C220 700 560 704 1024 780V1024H0Z" fill="#F2C12E"/>')
    parts.append(f'<g transform="translate({tx:.1f} {ty:.1f}) scale({s})">{h}</g>')
    parts.append("</g>")
    return "\n".join(parts)


def wordmark(theme_name):
    dark = theme_name == "dark"
    ink = PAPER if dark else FEATHER
    sub = "#B5AB99" if dark else INK2
    x, y, w, h, d = text_path("พิมพ์เสียง", 100)
    ex, ey, ew, eh, ed = text_path("PIMSIANG", 22, 7.5)
    pad = 24
    total_w = max(w, ew)
    width = total_w + 2 * pad
    thai_ty = pad - y
    en_y = pad + h + 20
    en_tx = pad + (total_w - ew) / 2 - ex
    en_ty = en_y - ey
    height = en_y + eh + pad
    body = (
        f'<path fill="{ink}" transform="translate({pad + (total_w - w) / 2 - x:.2f} {thai_ty:.2f})" d="{d}"/>\n'
        f'<path fill="{sub}" transform="translate({en_tx:.2f} {en_ty:.2f})" d="{ed}"/>'
    )
    return svg((round(width), round(height)), body, "PimSiang พิมพ์เสียง")


def write(path, text):
    path = BRAND / path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def main():
    if not RENDER.exists():
        sys.exit("build docs/brand/source/render first (render.sh does this)")
    poses = ["default", "listening", "writing", "note", "confused", "sleeping", "waving", "oops", "searching"]
    for variant, theme in (("", LIGHT), ("-dark", DARK)):
        for name in poses:
            write(f"mascot/pimsiang-myna-{name}{variant}.svg", svg((120, 120), pose(name, theme), f"PimSiang hill myna, {name}"))
        for state in ["default", "listening", "writing", "confused"]:
            write(f"mascot/pimsiang-myna-head-{state}{variant}.svg", svg((64, 64), head(state, theme), f"PimSiang hill myna head, {state}"))
    write("pimsiang-icon.svg", svg((1024, 1024), icon(False), "PimSiang app icon"))
    write("pimsiang-icon-maskable.svg", svg((1024, 1024), icon(True), "PimSiang app icon, full bleed"))
    for state in ["", "listening", "transcribing", "attention"]:
        suffix = f"-{state}" if state else ""
        write(f"pimsiang-menubar{suffix}.svg", svg((18, 18), menubar(state), f"PimSiang menu bar glyph {state}".strip()))
    write("pimsiang-wordmark.svg", wordmark("light"))
    write("pimsiang-wordmark-dark.svg", wordmark("dark"))


main()
