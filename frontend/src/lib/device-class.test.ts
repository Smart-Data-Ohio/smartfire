import { afterEach, describe, expect, it, vi } from "vitest";
import { browserDeviceClass } from "./device-class.ts";

describe("browser device class", () => {
  afterEach(() => vi.unstubAllGlobals());

  it.each([
    [
      "Android desktop mode",
      "Mozilla/5.0 (X11; Linux x86_64)",
      "Linux armv8l",
      5,
      "coarse",
      "none",
      "mobile",
    ],
    [
      "iPadOS desktop mode",
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15)",
      "MacIntel",
      5,
      "coarse",
      "none",
      "mobile",
    ],
    [
      "touch laptop",
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
      "Win32",
      10,
      "fine",
      "hover",
      "desktop",
    ],
    ["desktop", "Mozilla/5.0 (X11; Linux x86_64)", "Linux x86_64", 0, "fine", "hover", "desktop"],
    [
      "phone",
      "Mozilla/5.0 (Linux; Android 15; Pixel 9)",
      "Linux armv8l",
      5,
      "coarse",
      "none",
      "mobile",
    ],
    ["fine pointer with phone UA", "Mozilla/5.0 (iPhone)", "iPhone", 5, "fine", "hover", "desktop"],
  ])(
    "classifies %s from its primary pointer",
    (_name, userAgent, platform, maxTouchPoints, pointer, hover, expected) => {
      vi.stubGlobal(
        "matchMedia",
        vi.fn((query: string) => ({
          matches: query === `(pointer: ${pointer}) and (hover: ${hover})`,
        })),
      );
      expect(browserDeviceClass({ userAgent, platform, maxTouchPoints })).toBe(expected);
    },
  );

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
