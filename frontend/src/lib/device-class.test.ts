import { describe, expect, it } from "vitest";
import { browserDeviceClass } from "./device-class.ts";

describe("browser device class", () => {
  it.each([
    ["Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)", "iPhone", 5, "mobile"],
    ["Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15)", "MacIntel", 5, "mobile"],
    ["Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15)", "MacIntel", 0, "desktop"],
    ["Mozilla/5.0 (Linux; Android 15; Pixel 9)", "Linux armv8l", 5, "mobile"],
    ["Mozilla/5.0 (Windows NT 10.0; Win64; x64)", "Win32", 10, "desktop"],
    ["Mozilla/5.0 (X11; Linux x86_64)", "Linux x86_64", 0, "desktop"],
    ["", "", 0, "unknown"],
  ])(
    "classifies %s on %s with %s touch points as %s",
    (userAgent, platform, maxTouchPoints, expected) => {
      expect(browserDeviceClass({ userAgent, platform, maxTouchPoints })).toBe(expected);
    },
  );
});
