"""Regenerate app icons and a comparison sheet (requires Pillow).

Run from any directory: python logmancer-desktop/icons/generate.py
"""

from pathlib import Path

from PIL import Image, ImageDraw


HERE = Path(__file__).resolve().parent
PUBLIC = HERE.parents[1] / "logmancer-web" / "public"
SIZES = (16, 24, 32, 48, 64, 128, 256, 512, 1024)


def prepare(path):
    image = Image.open(path).convert("RGBA")
    # Ignore almost transparent stray pixels when finding the artwork bounds.
    bounds = image.getchannel("A").point(lambda value: 255 if value > 16 else 0).getbbox()
    if bounds is None:
        raise ValueError(f"Empty icon: {path}")
    image = image.crop(bounds)
    side = round(max(image.size) / 0.92)
    canvas = Image.new("RGBA", (side, side))
    canvas.alpha_composite(image, ((side - image.width) // 2, (side - image.height) // 2))
    return canvas


def resized(image, size):
    return image.resize((size, size), Image.Resampling.LANCZOS)


def main():
    original = prepare(HERE / "source.png")
    simplified = prepare(HERE / "source-small.png")
    variants = {size: resized(simplified if size <= 64 else original, size) for size in SIZES}
    PUBLIC.mkdir(parents=True, exist_ok=True)

    for size in (16, 32, 48):
        variants[size].save(PUBLIC / f"favicon-{size}x{size}.png")
    variants[48].save(
        PUBLIC / "favicon.ico", sizes=[(size, size) for size in (16, 32, 48)],
        append_images=[variants[16], variants[32]],
    )

    for name, size in (("32x32.png", 32), ("128x128.png", 128),
                       ("128x128@2x.png", 256), ("icon.png", 512)):
        variants[size].save(HERE / name)
    variants[256].save(
        HERE / "icon.ico", sizes=[(size, size) for size in SIZES if size <= 256],
        append_images=[image for size, image in variants.items() if size < 256],
    )
    variants[1024].save(
        HERE / "icon.icns", append_images=list(variants.values()),
    )
    # Keep the existing Windows/Store icon set consistent with the new artwork.
    for path in sorted(HERE.glob("Square*Logo.png")) + [HERE / "StoreLogo.png"]:
        with Image.open(path) as image:
            size = image.width
        resized(simplified if size <= 64 else original, size).save(path)

    sheet = Image.new("RGB", (880, 740))
    draw = ImageDraw.Draw(sheet)
    for row, (background, foreground) in enumerate((("#181a1f", "#e5e7eb"), ("#f7f9fc", "#111827"))):
        y = row * 370
        draw.rectangle((0, y, 880, y + 369), fill=background)
        draw.text((20, y + 12), "Logmancer - actual pixel sizes / tamanos reales", fill=foreground)
        for line, (label, source) in enumerate((("Simplified / simplificado", simplified), ("Original", original))):
            draw.text((20, y + 64 + line * 150), label, fill=foreground)
            for column, size in enumerate((16, 24, 32, 48, 64, 128)):
                x = 220 + column * 105
                sample = resized(source, size)
                top = y + 50 + line * 150
                sheet.paste(sample, (x, top), sample)
                draw.text((x, top + size + 5), f"{size}px", fill=foreground)
    preview = HERE / "preview.png"
    sheet.save(preview)
    print(f"Generated Web/Desktop icons. Comparison: {preview}")


if __name__ == "__main__":
    main()
