"""Regenerates Merlo's Android launcher icons from logo.svg.

    pip install cairosvg pillow && python3 crates/app/assets/brand/make_icons.py

Writes crates/app/android/res/, which the deploy workflow copies over dx's
default resources (dx 0.7 ignores the configured icon on Android).
"""

from pathlib import Path

import cairosvg
from PIL import Image, ImageDraw

HERE = Path(__file__).parent
RES = HERE.parent.parent / "android" / "res"
LOGO = (HERE / "logo.svg").read_text()
BACKGROUND = "#fbf7ef"
DENSITIES = {"mdpi": 1, "hdpi": 1.5, "xhdpi": 2, "xxhdpi": 3, "xxxhdpi": 4}


def render(svg: str, px: int) -> Image.Image:
    path = RES / ".tmp.png"
    cairosvg.svg2png(bytestring=svg.encode(), write_to=str(path), output_width=px, output_height=px)
    image = Image.open(path).convert("RGBA")
    path.unlink()
    return image


def framed(scale: float, background: bool) -> str:
    """The logo shrunk about its centre; adaptive icons crop to a 66/108 safe zone."""
    inner = LOGO.split(">", 1)[1].rsplit("</svg>", 1)[0]
    fill = f'<rect width="108" height="108" fill="{BACKGROUND}"/>' if background else ""
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 108 108">'
        f'{fill}<g transform="translate(54 54) scale({scale}) translate(-54 -54)">{inner}</g></svg>'
    )


def main() -> None:
    for name, density in DENSITIES.items():
        folder = RES / f"mipmap-{name}"
        folder.mkdir(parents=True, exist_ok=True)
        # Adaptive foreground: 108 dp, transparent, logo inside the safe zone.
        render(framed(0.74, background=False), round(108 * density)).save(folder / "ic_launcher_fg.png")
        # Legacy icon (Android 7): 48 dp, round.
        size = round(48 * density)
        icon = render(framed(0.9, background=True), size)
        mask = Image.new("L", (size, size), 0)
        ImageDraw.Draw(mask).ellipse((0, 0, size - 1, size - 1), fill=255)
        round_icon = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        round_icon.paste(icon, (0, 0), mask)
        round_icon.save(folder / "ic_launcher.webp", "WEBP", lossless=True)

    (RES / "mipmap-anydpi-v26").mkdir(parents=True, exist_ok=True)
    (RES / "mipmap-anydpi-v26" / "ic_launcher.xml").write_text(
        '<?xml version="1.0" encoding="utf-8"?>\n'
        '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n'
        '    <background android:drawable="@color/ic_launcher_background" />\n'
        '    <foreground android:drawable="@mipmap/ic_launcher_fg" />\n'
        '    <monochrome android:drawable="@mipmap/ic_launcher_fg" />\n'
        "</adaptive-icon>\n"
    )
    (RES / "values").mkdir(parents=True, exist_ok=True)
    (RES / "values" / "ic_launcher_background.xml").write_text(
        f'<?xml version="1.0" encoding="utf-8"?>\n<resources>\n'
        f'    <color name="ic_launcher_background">{BACKGROUND}</color>\n</resources>\n'
    )


if __name__ == "__main__":
    main()
