import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { CustomTokens } from "../../lib/custom-palette.ts";
import { PaletteEditor } from "./palette-editor.tsx";

const accent = () => document.documentElement.style.getPropertyValue("--accent-solid");

beforeEach(() => {
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  document.documentElement.removeAttribute("style");
});

it("keeps a hex as typed and saves the whole colour, not the 3-digit prefix", async () => {
  const user = userEvent.setup();
  const saved: (CustomTokens | null)[] = [];

  render(
    <PaletteEditor
      disabled={false}
      onSave={async (tokens) => {
        saved.push(tokens);
      }}
    />,
  );

  const hex = screen.getByRole("textbox", { name: "Accent hex value" });

  await user.clear(hex);
  await user.type(hex, "#123");
  expect(hex).toHaveProperty("value", "#123");
  await user.type(hex, "456");
  expect(hex).toHaveProperty("value", "#123456");
  expect(accent()).toBe("#123456");

  await user.click(screen.getByRole("button", { name: "Save colours" }));
  expect(saved).toEqual([expect.objectContaining({ "--accent-solid": "#123456" })]);
});

it("marks an incomplete hex invalid on blur and keeps the last good colour", async () => {
  const user = userEvent.setup();

  render(<PaletteEditor disabled={false} onSave={async () => undefined} />);

  const hex = screen.getByRole("textbox", { name: "Accent hex value" });

  await user.clear(hex);
  await user.type(hex, "#12345");
  await user.tab();
  expect(hex.getAttribute("aria-invalid")).toBe("true");
  expect(screen.getByRole("alert").textContent).toMatch(/hex colour/);
  expect(accent()).toBe("");

  // Enter accepts a short hex, expanded.
  await user.clear(hex);
  await user.type(hex, "#abc{Enter}");
  expect(hex).toHaveProperty("value", "#aabbcc");
  expect(accent()).toBe("#aabbcc");
});

it("ignores a save from an earlier editor that finishes after the page was reopened", async () => {
  const user = userEvent.setup();
  let finish: () => void = () => undefined;

  const first = render(
    <PaletteEditor
      disabled={false}
      onSave={() =>
        new Promise<void>((resolve) => {
          finish = resolve;
        })
      }
    />,
  );

  const hex = () => screen.getByRole("textbox", { name: "Accent hex value" });

  await user.clear(hex());
  await user.type(hex(), "#111111");
  await user.click(screen.getByRole("button", { name: "Save colours" }));
  first.unmount();

  render(<PaletteEditor disabled={false} onSave={async () => undefined} />);
  await user.clear(hex());
  await user.type(hex(), "#222222");
  expect(accent()).toBe("#222222");

  await act(async () => finish());
  expect(accent()).toBe("#222222");
});
