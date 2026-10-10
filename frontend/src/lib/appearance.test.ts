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

/** What's stored: this device's choices, plus the chosen palette's derived tokens. */
type Stored = Partial<Appearance> & {
  readonly paletteTokens?: Readonly<Partial<Record<`--${string}`, string>>>;
};

function stored(): Stored {
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
  it("loads personal account preferences without storing them as device overrides", async () => {
    inlineBoot("dark", "default");
    const boot = document.getElementById("boot");

    if (boot === null) throw new Error("missing boot");
    boot.textContent = JSON.stringify({
      theme: "dark",
      textSize: "default",
      appearancePreferences: {
        version: 1,
        palette: "ocean",
        font: "serif",
        density: "compact",
        motion: "reduce",
      },
    });
    const { restoreAppearance, appearanceSnapshot } = await load();
    restoreAppearance();
    expect(appearanceSnapshot()).toMatchObject({
      palette: "ocean",
      font: "serif",
      density: "compact",
      motion: "reduce",
    });
    expect(stored()).not.toHaveProperty("palette", "ocean");
  });

  it("keeps an explicit device palette over the account preference", async () => {
    localStorage.setItem(KEY, JSON.stringify({ palette: "ember" }));
    inlineBoot("system", "default");
    const boot = document.getElementById("boot");

    if (boot === null) throw new Error("missing boot");
    boot.textContent = JSON.stringify({
      theme: "system",
      textSize: "default",
      appearancePreferences: { version: 1, palette: "ocean", font: "serif" },
    });
    const { restoreAppearance, appearanceSnapshot } = await load();
    restoreAppearance();
    expect(appearanceSnapshot()).toMatchObject({ palette: "ember", font: "serif" });
  });
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

  it("takes no account's theme from an earlier visit: without boot, the OS decides", async () => {
    // What an earlier person's account left behind, in either earlier format.
    localStorage.setItem(
      KEY,
      JSON.stringify({ theme: "dark", accountTheme: "dark", textSize: "large" }),
    );
    const { restoreAppearance } = await load();

    restoreAppearance();

    expect(html().dataset.theme).toBeUndefined();
    expect(html().dataset.textSize).toBe("default");
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

  it("ignores corrupted values that only read as a choice once turned into text", async () => {
    // Each would read as a real choice through String(): ["dark"] is "dark".
    localStorage.setItem(
      KEY,
      JSON.stringify({
        themeOverride: ["dark"],
        density: ["compact"],
        motion: ["reduce"],
        palette: ["ocean"],
        font: ["serif"],
      }),
    );
    const boot = document.createElement("script");

    boot.type = "application/json";
    boot.id = "boot";
    boot.textContent = JSON.stringify({ theme: ["light"], textSize: ["larger"] });
    document.head.append(boot);
    const { restoreAppearance, appearanceSnapshot } = await load();

    restoreAppearance();

    expect(appearanceSnapshot()).toMatchObject({
      theme: "system",
      themeOverride: null,
      accountTheme: "system",
      textSize: "default",
      density: "comfortable",
      motion: "system",
      palette: "smartfire",
      font: "inter",
    });
    expect(html().dataset.theme).toBeUndefined();
    expect(html().dataset.textSize).toBe("default");
    expect(html().dataset.density).toBeUndefined();
    expect(html().dataset.palette).toBeUndefined();
  });
});

describe("applyAccountAppearance", () => {
  it("keeps failed device edits through later edits and merges another tab's stored choices", async () => {
    localStorage.setItem(KEY, JSON.stringify({ palette: "ocean" }));
    const a = await load();
    a.restoreAppearance();

    const writes = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new DOMException("Quota exceeded", "QuotaExceededError");
    });

    try {
      a.setPalette("ember");
      a.setThemeOverride("light");
      a.setFont("mono");
      expect(a.appearanceSnapshot()).toMatchObject({
        palette: "ember",
        theme: "light",
        font: "mono",
      });
      writes.mockRestore();
      localStorage.setItem(KEY, JSON.stringify({ palette: "ocean", motion: "reduce" }));
      window.dispatchEvent(new StorageEvent("storage", { key: KEY, storageArea: localStorage }));
      expect(a.appearanceSnapshot()).toMatchObject({
        palette: "ember",
        theme: "light",
        font: "mono",
        motion: "reduce",
      });
      a.setDensity("compact");
      expect(stored()).toMatchObject({
        palette: "ember",
        themeOverride: "light",
        font: "mono",
        motion: "reduce",
        density: "compact",
      });
      localStorage.setItem(KEY, JSON.stringify({ palette: "forest" }));
      window.dispatchEvent(new StorageEvent("storage", { key: KEY, storageArea: localStorage }));
      expect(a.appearanceSnapshot().palette).toBe("forest");
    } finally {
      writes.mockRestore();
    }
  });

  it("keeps an unsaved override removal instead of restoring the stale pin", async () => {
    localStorage.setItem(KEY, JSON.stringify({ palette: "ember", themeOverride: "light" }));
    const a = await load();
    a.restoreAppearance();

    const writes = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new DOMException("Quota exceeded", "QuotaExceededError");
    });

    try {
      a.setPersonalAppearanceOverride(false);
      a.setThemeOverride(null);
      a.setFont("mono");
      expect(a.appearanceSnapshot()).toMatchObject({
        palette: "smartfire",
        theme: "system",
        font: "mono",
      });
    } finally {
      writes.mockRestore();
    }
  });

  it("keeps tab A's Ember pin after tab B's account save finishes and A reloads", async () => {
    const a = await load();
    const b = await load();
    a.restoreAppearance();
    b.restoreAppearance();
    a.setPalette("ember");
    b.applyAccountAppearance({
      theme: "dark",
      textSize: "default",
      appearancePreferences: { version: 1, palette: "ocean" },
    });
    const reloaded = await load();
    reloaded.restoreAppearance();
    expect(reloaded.appearanceSnapshot().palette).toBe("ember");
  });

  it("changes only the requested override after reading another tab's choices", async () => {
    const a = await load();
    const b = await load();
    a.restoreAppearance();
    b.restoreAppearance();
    a.setPalette("ember");
    a.setThemeOverride("light");
    b.setFont("mono");
    expect(stored()).toMatchObject({ palette: "ember", themeOverride: "light", font: "mono" });
    a.setMotion("reduce");
    expect(stored()).toMatchObject({
      palette: "ember",
      themeOverride: "light",
      font: "mono",
      motion: "reduce",
    });
  });

  it("applies other tabs' storage changes and clears without writing back", async () => {
    const b = await load();
    b.restoreAppearance();
    b.applyAccountAppearance({
      theme: "dark",
      textSize: "default",
      appearancePreferences: { version: 1, palette: "ocean" },
    });
    const writes = vi.spyOn(Storage.prototype, "setItem");
    localStorage.setItem(
      KEY,
      JSON.stringify({ palette: "ember", font: "mono", themeOverride: "light" }),
    );
    writes.mockClear();
    window.dispatchEvent(new StorageEvent("storage", { key: KEY, storageArea: localStorage }));
    expect(b.appearanceSnapshot()).toMatchObject({
      palette: "ember",
      font: "mono",
      theme: "light",
    });
    expect(writes).not.toHaveBeenCalled();
    localStorage.clear();
    window.dispatchEvent(new StorageEvent("storage", { key: null, storageArea: localStorage }));
    expect(b.appearanceSnapshot()).toMatchObject({
      palette: "ocean",
      font: "inter",
      theme: "dark",
    });
    expect(writes).not.toHaveBeenCalled();
    writes.mockRestore();
  });

  it("shows the account's theme and size without writing device storage", async () => {
    const { applyAccountAppearance } = await load();

    applyAccountAppearance({ theme: "dark", textSize: "smaller" });

    expect(html().dataset.theme).toBe("dark");
    expect(html().dataset.textSize).toBe("smaller");
    // Only this device's own choices are stored: the classic pages read the pin, and the next
    // person to sign in on this browser must not inherit this account's theme.
    expect(localStorage.getItem(KEY)).toBeNull();
  });

  it("leaves a device's pinned theme on screen", async () => {
    const { applyAccountAppearance, setThemeOverride } = await load();

    setThemeOverride("light");
    applyAccountAppearance({ theme: "dark", textSize: "default" });

    expect(html().dataset.theme).toBe("light");
    expect(stored()).toEqual({
      themeOverride: "light",
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
    expect(stored()).toMatchObject({ themeOverride: null });
    expect(stored()).not.toHaveProperty("theme");
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
    // Only the name: index.html carries every palette's tokens, built from the same presets.
    expect(stored()).not.toHaveProperty("paletteTokens");
  });

  it("ignores tokens an earlier version stored, and drops them at the next change", async () => {
    localStorage.setItem(
      KEY,
      JSON.stringify({
        palette: "ember",
        paletteTokens: { "--text-body": "0px", "--accent": "light-dark(red, red)" },
      }),
    );
    const { restoreAppearance, setFont } = await load();
    const { paletteTokens } = await import("./palette.ts");

    restoreAppearance();

    expect(html().style.getPropertyValue("--text-body")).toBe("");
    expect(html().style.getPropertyValue("--accent")).toBe(paletteTokens("ember").get("--accent"));

    setFont("serif");

    expect(stored()).toMatchObject({ palette: "ember", font: "serif" });
    expect(stored()).not.toHaveProperty("paletteTokens");
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
    localStorage.setItem(
      KEY,
      JSON.stringify({ accountTheme: "light", palette: "neon", font: "comic" }),
    );
    const { restoreAppearance, appearanceSnapshot } = await load();

    restoreAppearance();

    expect(html().dataset.palette).toBeUndefined();
    expect(html().dataset.font).toBeUndefined();
    expect(appearanceSnapshot()).toMatchObject({ palette: "smartfire", font: "inter" });
  });
});
