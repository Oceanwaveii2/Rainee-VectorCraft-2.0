"""Constrain a neural appearance to a soft corridor around authored stroke guidance."""
import numpy as np
from PIL import Image, ImageFilter, ImageOps


def constrain_appearance(rgba, guidance):
    if rgba.dtype != np.uint8 or rgba.shape != (guidance.height, guidance.width, 4):
        raise ValueError("Appearance and guidance dimensions must match")
    # 12 preview pixels of dilation, then a bounded soft edge. This is an adapter
    # constraint, not part of the author model and not a natural-media simulation.
    support = ImageOps.invert(guidance.convert("L")).filter(
        ImageFilter.MaxFilter(25)
    ).filter(ImageFilter.GaussianBlur(2))
    mask = np.asarray(support, dtype=np.uint16)
    out = rgba.copy()
    clipped = int(np.count_nonzero((out[:, :, 3] > 0) & (mask == 0)))
    out[:, :, 3] = (out[:, :, 3].astype(np.uint16) * mask // 255).astype(np.uint8)
    out[out[:, :, 3] == 0, :3] = 0
    return out, clipped
