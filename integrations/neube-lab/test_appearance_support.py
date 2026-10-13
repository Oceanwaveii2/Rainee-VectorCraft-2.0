import unittest
import numpy as np
from PIL import Image, ImageDraw
from appearance_support import constrain_appearance


class AppearanceSupportTests(unittest.TestCase):
    def test_background_removed_and_stroke_center_preserved(self):
        guidance = Image.new("L", (128, 96), 255)
        ImageDraw.Draw(guidance).line([(20, 48), (108, 48)], fill=0, width=5)
        source = np.full((96, 128, 4), 160, dtype=np.uint8)
        source[:, :, 3] = 255
        out, clipped = constrain_appearance(source, guidance)
        self.assertGreater(clipped, 0)
        self.assertEqual(int(out[48, 64, 3]), 255)
        self.assertEqual(int(out[0, 64, 3]), 0)
        self.assertTrue(np.all(out[0, 64, :3] == 0))
        self.assertTrue(np.array_equal(source[:, :, 3], np.full((96, 128), 255)))

    def test_mismatched_image_rejected(self):
        with self.assertRaises(ValueError):
            constrain_appearance(np.zeros((10, 10, 4), dtype=np.uint8), Image.new("L", (20, 20)))


if __name__ == "__main__":
    unittest.main()
