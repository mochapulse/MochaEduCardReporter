#!/usr/bin/env python3

from pathlib import Path

from PIL import Image, ImageDraw


ROOT_DIR = Path(__file__).resolve().parent.parent
ASSETS_DIR = ROOT_DIR / "assets"
PNG_PATH = ASSETS_DIR / "app.png"
ICO_PATH = ASSETS_DIR / "app.ico"


def rounded_rectangle(draw: ImageDraw.ImageDraw, box: tuple[int, int, int, int], radius: int, fill: str) -> None:
    draw.rounded_rectangle(box, radius=radius, fill=fill)


def generate_icon() -> None:
    ASSETS_DIR.mkdir(parents=True, exist_ok=True)

    size = 1024
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)

    rounded_rectangle(draw, (64, 64, 960, 960), 220, "#0f172a")
    rounded_rectangle(draw, (118, 118, 906, 906), 178, "#1d4ed8")
    rounded_rectangle(draw, (180, 170, 700, 794), 72, "#f8fafc")
    rounded_rectangle(draw, (228, 252, 652, 314), 28, "#94a3b8")
    rounded_rectangle(draw, (228, 384, 652, 446), 28, "#22c55e")
    rounded_rectangle(draw, (228, 516, 652, 578), 28, "#7c3aed")
    rounded_rectangle(draw, (228, 648, 540, 710), 28, "#ef4444")

    cup_box = (578, 484, 844, 746)
    draw.ellipse(cup_box, fill="#f59e0b")
    draw.ellipse((626, 530, 796, 676), fill="#0f172a")
    draw.arc((772, 548, 918, 700), start=-70, end=70, fill="#f8fafc", width=40)

    image.save(PNG_PATH)
    image.save(
        ICO_PATH,
        sizes=[
            (16, 16),
            (24, 24),
            (32, 32),
            (48, 48),
            (64, 64),
            (128, 128),
            (256, 256),
        ],
    )


if __name__ == "__main__":
    generate_icon()
    print(f"Wrote {PNG_PATH}")
    print(f"Wrote {ICO_PATH}")
