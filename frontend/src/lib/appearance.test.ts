import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Appearance } from "./appearance.ts";

const KEY = "smartfire.appearance";

/** A fresh copy of the module: its remembered state starts over for each test. */
async function load() {
  vi.resetModules();

  return import("./appearance.ts");
}

/** The shell's inline boot JSON, as the Rust shell renders it. */
function inlineBoot(theme: string, textSize: string): void {
  const script = document.createElement("script");

  script.type = "application/json";
  script.id = "boot";
  script.textContent = JSON.stringify({
    user: { id: 1, name: "Riel", avatarUrl: "/a.svg" },
    account: { name: "Smart Data" },
    theme,
    textSize,
  });
  document.head.append(script);
}

function stored(): Partial<Appearance> {
  return JSON.parse(localStorage.getItem(KEY) ?? "{}");
}

const html = () => document.documentElement;

beforeEach(() => {
  localStorage.clear();
  document.getElementById("boot")?.remove();

  for (const name of ["theme", "textSize", "density", "motion", "palette", "font"]) {
    delete html().dataset[name];
  }

  html().removeAttribute("style");
});

afterEach(() => {
  document.getElementById("boot")?.remove();
});

describe("restoreAppearance", () => {
  it("applies the account's theme and text size from the inline boot before the first render", async () => {
    inlineBoot("dark", "larger");
    const { restoreAppearance } = await load();

    restoreAppearance();

    expect(html().dataset.theme).toBe("dark");
    expect(html().dataset.textSize).toBe("larger");
  });

  it("lets the account's theme replace a stale theme this device remembered", async () => {
    // What the device stored before the account theme applied on boot: a bare theme.
    localStorage.setItem(KEY, JSON.stringify({ theme: "light", density: "compact" }));
    inlineBoot("dark", "default");
    const { restoreAppearance } = await load();

    restoreAppearance();

    expect(html().dataset.theme).toBe("dark");
    expect(html().dataset.density).toBe("compact");
  });

  it("keeps a theme pinned on this device over the account's", async () => {
    localStorage.setItem(KEY, JSON.stringify({ themeOverride: "light", accountTheme: "dark" }));
    inlineBoot("dark", "small");
    const { restoreAppearance } = await load();

    restoreAppearance();

    expect(html().dataset.theme).toBe("light");
    expect(html().dataset.textSize).toBe("small");
  });

  it("uses the account's last known choices until boot arrives", async () => {
    localStorage.setItem(KEY, JSON.stringify({ accountTheme: "dark", textSize: "large" }));
    const { restoreAppearance } = await load();

    restoreAppearance();

    expect(html().dataset.theme).toBe("dark");
    expect(html().dataset.textSize).toBe("large");
  });

  it("falls back to the system theme and default size with nothing known", async () => {
    const { restoreAppearance } = await load();

    restoreAppearance();

    expect(html().dataset.theme).toBeUndefined();
    expect(html().dataset.textSize).toBe("default");
  });

  it("ignores values it doesn't know", async () => {
    inlineBoot("sepia", "huge");
    const { restoreAppearance } = await load();

    restoreAppearance();

    expect(html().dataset.theme).toBeUndefined();
    expect(html().dataset.textSize).toBe("default");
  });
});

describe("applyAccountAppearance", () => {
  it("shows the account's theme and size, and remembers them for the next start", async () => {
    const { applyAccountAppearance } = await load();

    applyAccountAppearance({ theme: "dark", textSize: "smaller" });

    expect(html().dataset.theme).toBe("dark");
    expect(html().dataset.textSize).toBe("smaller");
    // `theme` is what's on screen: the classic pages' script reads it.
    expect(stored()).toMatchObject({ theme: "dark", accountTheme: "dark", textSize: "smaller" });
  });

  it("leaves a device's pinned theme on screen", async () => {
    const { applyAccountAppearance, setThemeOverride } = await load();

    setThemeOverride("light");
    applyAccountAppearance({ theme: "dark", textSize: "default" });

    expect(html().dataset.theme).toBe("light");
    expect(stored()).toMatchObject({
      theme: "light",
      themeOverride: "light",
      accountTheme: "dark",
    });
  });
});

describe("setThemeOverride", () => {
  it("pins a theme on this device, and clearing it goes back to the account's", async () => {
    const { applyAccountAppearance, setThemeOverride } = await load();

    applyAccountAppearance({ theme: "dark", textSize: "default" });
    setThemeOverride("system");
    expect(html().dataset.theme).toBeUndefined();

    setThemeOverride(null);
    expect(html().dataset.theme).toBe("dark");
    expect(stored()).toMatchObject({ theme: "dark", themeOverride: null });
  });
});

describe("appearanceSnapshot", () => {
  it("reports the theme on screen, the device's pin and the account's choices", async () => {
    const { applyAccountAppearance, setThemeOverride, appearanceSnapshot } = await load();

    applyAccountAppearance({ theme: "dark", textSize: "large" });
    setThemeOverride("light");

    expect(appearanceSnapshot()).toMatchObject({
      theme: "light",
      themeOverride: "light",
      accountTheme: "dark",
      textSize: "large",
    });
  });
});

describe("palette and font", () => {
  it("paints a palette's tokens on the page and remembers it for this device", async () => {
    const { setPalette } = await load();

    setPalette("ember");

    expect(html().dataset.palette).toBe("ember");
    expect(html().style.getPropertyValue("--bg-pane")).toMatch(/^light-dark\(oklch/);
    expect(html().style.getPropertyValue("--accent")).toMatch(/^light-dark\(oklch/);
    expect(stored()).toMatchObject({ palette: "ember" });
  });

  it("goes back to the stylesheet's colours with Smartfire's palette", async () => {
    const { setPalette } = await load();

    setPalette("forest");
    setPalette("smartfire");

    expect(html().dataset.palette).toBeUndefined();
    expect(html().style.getPropertyValue("--bg-pane")).toBe("");
  });

  it("sets the font on the page, Inter being the default", async () => {
    const { setFont } = await load();

    setFont("atkinson");
    expect(html().dataset.font).toBe("atkinson");

    setFont("inter");
    expect(html().dataset.font).toBeUndefined();
    expect(stored()).toMatchObject({ font: "inter" });
  });

  it("restores this device's palette and font before the first render", async () => {
    localStorage.setItem(
      KEY,
      JSON.stringify({ accountTheme: "dark", palette: "ocean", font: "serif" }),
    );
    inlineBoot("dark", "default");
    const { restoreAppearance } = await load();

    restoreAppearance();

    expect(html().dataset.palette).toBe("ocean");
    expect(html().style.getPropertyValue("--accent")).not.toBe("");
    expect(html().dataset.font).toBe("serif");
  });

  it("ignores a palette or font it doesn't know", async () => {
    localStorage.setItem(KEY, JSON.stringify({ accountTheme: "light", palette: "neon", font: "comic" }));
    const { restoreAppearance, appearanceSnapshot } = await load();

    restoreAppearance();

    expect(html().dataset.palette).toBeUndefined();
    expect(html().dataset.font).toBeUndefined();
    expect(appearanceSnapshot()).toMatchObject({ palette: "smartfire", font: "inter" });
  });
});
