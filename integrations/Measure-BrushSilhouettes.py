"""Image analysis only: compare normalized binary silhouettes, never edit references."""
import json
from pathlib import Path
from PIL import Image

ROOT = Path("D:/DevData/Rainee/verification/owner-art-brushes-2026-10-11")


def mask(path):
    with Image.open(path) as source:
        image = source.convert("RGBA")
        rgb = Image.new("RGB", image.size, "white")
        rgb.paste(image, mask=image.getchannel("A"))
        binary = rgb.convert("L").point(lambda value: 255 if value < 190 else 0)
        bounds = binary.getbbox()
        if bounds is None:
            raise ValueError("Empty silhouette")
        return binary.crop(bounds).resize((1024, 128), Image.Resampling.NEAREST)


results = []
for index in range(1, 10):
    source = mask(ROOT / f"source-{index:02d}.png")
    rendered = mask(ROOT / f"straight-{index:02d}.png")
    pairs = list(zip(source.get_flattened_data(), rendered.get_flattened_data()))
    intersection = sum(a != 0 and b != 0 for a, b in pairs)
    union = sum(a != 0 or b != 0 for a, b in pairs)
    results.append({"brush": index, "normalizedSilhouetteIoU": intersection / max(1, union)})
duplicates = []
for zero, sketch in ((1, 5), (2, 7), (3, 8)):
    path = ROOT / f"zero-source-{zero:02d}.png"
    if not path.exists():
        continue
    pairs = list(zip(mask(path).get_flattened_data(), mask(ROOT / f"source-{sketch:02d}.png").get_flattened_data()))
    intersection = sum(a != 0 and b != 0 for a, b in pairs)
    union = sum(a != 0 or b != 0 for a, b in pairs)
    duplicates.append({"zeroBrush": zero, "sketchBrush": sketch, "normalizedSilhouetteIoU": intersection / max(1, union)})
print(json.dumps({"results": results, "zeroFileCorrespondence": duplicates, "scope": "agreement with recovered monochrome SVG candidate, not original application output or dynamic behavior"}))
