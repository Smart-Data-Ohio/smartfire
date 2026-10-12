These tiny fixtures were generated locally with Python standard-library struct and zlib.

The static and animated images have a 2 by 3 canvas, with a black first frame and a white second frame. The hidden-default APNG has an excluded red default image and a 1 by 1 black first animation frame at offset 1,1. The boundary fixtures test the 320-pixel limit on each axis. Tests read these bytes directly; no image tools run to generate fixtures in tests.
