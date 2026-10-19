import type { DeviceClass } from "../gen/DeviceClass.ts";

type DeviceBrowser = Pick<Navigator, "userAgent" | "platform" | "maxTouchPoints">;

export function browserDeviceClass(browser: DeviceBrowser = navigator): DeviceClass {
  if (
    /iphone|ipad|ipod|android|mobile/i.test(browser.userAgent) ||
    (browser.platform === "MacIntel" && browser.maxTouchPoints > 1)
  ) {
    return "mobile";
  }

  return /windows|macintosh|x11|cros|linux/i.test(browser.userAgent) ? "desktop" : "unknown";
}
