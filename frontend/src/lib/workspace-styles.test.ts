import { afterEach, describe, expect, it } from "vitest";
import { applyEvents } from "../store/reducers.ts";
import { initialState } from "../store/state.ts";
import { mutations, store } from "../store/store.ts";
import { restoreAppearance, setFont, setPalette, setThemeOverride } from "./appearance.ts";
import {
  applyWorkspaceStyles,
  followWorkspaceStyles,
  previewWorkspaceStyles,
} from "./workspace-styles.ts";

const style = () => document.head.querySelector('style[data-turbo-track="reload"]');

afterEach(() => {
  applyWorkspaceStyles(null);
  store.setState(initialState, true);
  localStorage.clear();
  restoreAppearance();
});

describe("workspace styles", () => {
  it("preserves range media queries and attribute selectors in live updates", () => {
    const css = '@media (width < 720px) { [data-label="<雪"] { color: red; } }';

    applyWorkspaceStyles(css);

    expect(style()?.textContent).toBe(css);
  });

  it("encodes only case-insensitive style end tags with an HTML delimiter", () => {
    applyWorkspaceStyles(
      'body::after { content: "</StYlE> </STYLE /> </style\n> </stylesheet>"; }',
    );

    expect(style()?.textContent).toBe(
      'body::after { content: "\\3c /StYlE> \\3c /STYLE /> \\3c /style\n> </stylesheet>"; }',
    );
  });

  it("uses classic's element and shell encoding, without escaping CSS selectors or quotes", () => {
    applyWorkspaceStyles('body > .label::after { content: "</style>&"; }');

    expect(style()?.textContent).toBe('body > .label::after { content: "\\3c /style>&"; }');
    expect(style()?.outerHTML).toBe(
      '<style data-turbo-track="reload">body > .label::after { content: "\\3c /style>&"; }</style>',
    );
    applyWorkspaceStyles("");
    expect(style()).toBeNull();
  });

  it("keeps explicit palette, font and theme choices above workspace defaults", () => {
    restoreAppearance();
    setThemeOverride("dark");
    setFont("serif");
    setPalette("graphite");
    const accent = document.documentElement.style.getPropertyValue("--accent");

    applyWorkspaceStyles(":root { --accent: red; --font-sans: monospace; color-scheme: light; }");

    expect(document.documentElement.style.getPropertyValue("--accent")).toBe(accent);
    expect(accent).not.toBe("");
    expect(document.documentElement.style.getPropertyValue("--font-sans")).toBe(
      "var(--font-preset-serif)",
    );
    expect(document.documentElement.style.getPropertyValue("color-scheme")).toBe("dark");
    setPalette("smartfire");
    setFont("inter");
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("");
    expect(document.documentElement.style.getPropertyValue("--font-sans")).toBe("");
    expect(style()?.textContent).toContain("--accent: red");
  });

  it("applies boot, live updates and clearing through the store", () => {
    mutations.setBoot({
      user: { id: 1, name: "Riel", avatarUrl: "/avatar" },
      account: {
        name: "Smart Data",
        logoUrl: null,
        logoStillUrl: null,
        bannerUrl: null,
        bannerStillUrl: null,
        uploadLimitBytes: 100 * 1024 * 1024,
      },
      customStyles: "body { color: red; }",
      theme: "system",
      textSize: "default",
      cableUrl: "/cable",
      serviceWorkerUrl: null,
      version: "test",
      revision: null,
      appearancePreferences: null,
    });
    const stop = followWorkspaceStyles();

    try {
      expect(style()?.textContent).toBe("body { color: red; }");
      store.setState(
        applyEvents(
          store.getState(),
          [
            {
              seq: 1,
              topic: "user",
              type: "workspace.styles.updated",
              data: { css: "body { color: blue; }" },
            },
          ],
          0,
        ),
      );
      expect(style()?.textContent).toBe("body { color: blue; }");
      mutations.setWorkspaceStyles(null);
      expect(style()).toBeNull();
    } finally {
      stop();
    }
  });

  it("previews the actual style element and restores the latest live save", () => {
    applyWorkspaceStyles("body { color: red; }");
    const stop = previewWorkspaceStyles('body::after { content: "</style>"; }');

    expect(style()?.textContent).toBe('body::after { content: "\\3c /style>"; }');
    applyWorkspaceStyles("body { color: blue; }");
    expect(style()?.textContent).toContain("content:");
    stop();
    expect(style()?.textContent).toBe("body { color: blue; }");
  });
});
