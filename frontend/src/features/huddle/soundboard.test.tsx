import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SoundboardCatalog } from "../../gen/SoundboardCatalog.ts";
import { soundboard } from "../../sync/huddles.ts";
import { callStore, initialCallState } from "./call-store.ts";
import { loadSoundboardEnabled, loadSoundboardVolume } from "./engine/preferences.ts";
import { SoundboardButton, SoundboardPanel } from "./soundboard.tsx";
import { resetPlayCooldown } from "./soundboard-player.ts";
import { resetSoundboard, setSoundboardEnabled } from "./soundboard-store.ts";

const CATALOG: SoundboardCatalog = {
  sounds: [
    {
      id: 1,
      name: "Airhorn",
      emoji: "📯",
      volume: 1,
      uploaderId: 1,
      audioUrl: "/blobs/airhorn.wav",
      createdAt: "2026-10-11T12:00:00Z",
      updatedAt: "2026-10-11T12:00:00Z",
    },
    {
      id: 2,
      name: "Rimshot",
      emoji: null,
      volume: 0.6,
      uploaderId: 1,
      audioUrl: "/blobs/rimshot.wav",
      createdAt: "2026-10-11T12:00:00Z",
      updatedAt: "2026-10-11T12:00:00Z",
    },
  ],
  capacity: 48,
};

function inCall(canPublish = true) {
  callStore.setState(
    { ...initialCallState, phase: "connected", roomId: 8, identity: "me-tab", canPublish },
    true,
  );
}

beforeEach(() => {
  localStorage.clear();
  resetSoundboard();
  resetPlayCooldown();
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  callStore.setState(initialCallState, true);
});

function sounds(): HTMLElement {
  return screen.getByRole("list", { name: "Sounds" });
}

describe("the soundboard picker", () => {
  it("shows each sound's emoji and name, and plays one to the call", async () => {
    vi.spyOn(soundboard, "catalog").mockResolvedValue(CATALOG);

    const play = vi.spyOn(soundboard, "play").mockResolvedValue(undefined);

    inCall();
    render(<SoundboardPanel />);

    const airhorn = await screen.findByRole("button", { name: "Airhorn" });

    expect(within(airhorn).getByText("📯")).toBeTruthy();
    expect(within(sounds()).getByRole("button", { name: "Rimshot" }).textContent).toBe("🔊Rimshot");

    fireEvent.click(airhorn);
    expect(play).toHaveBeenCalledExactlyOnceWith(8, "me-tab", 1);
  });

  it("says so when there are no sounds", async () => {
    vi.spyOn(soundboard, "catalog").mockResolvedValue({ sounds: [], capacity: 48 });
    inCall();
    render(<SoundboardPanel />);

    expect(await screen.findByText(/No sounds yet\./)).toBeTruthy();
  });

  it("can't play while the viewer can't speak, and says why", async () => {
    vi.spyOn(soundboard, "catalog").mockResolvedValue(CATALOG);
    inCall(false);
    render(<SoundboardPanel />);

    const airhorn = await screen.findByRole("button", { name: "Airhorn" });

    expect(airhorn.hasAttribute("disabled")).toBe(true);
    expect(screen.getByText("You can’t play sounds while you can’t speak here.")).toBeTruthy();
  });

  it("with sounds off, explains it, disables the sounds and offers the switch back", async () => {
    vi.spyOn(soundboard, "catalog").mockResolvedValue(CATALOG);
    setSoundboardEnabled(false);
    inCall();
    render(<SoundboardPanel />);

    expect(
      screen.getByText(
        "Soundboard sounds are off. Turn them on below to hear sounds and play them.",
      ),
    ).toBeTruthy();
    expect((await screen.findByRole("button", { name: "Airhorn" })).hasAttribute("disabled")).toBe(
      true,
    );
    expect(screen.getByLabelText(/Soundboard volume/).hasAttribute("disabled")).toBe(true);

    fireEvent.click(screen.getByRole("switch", { name: /Play soundboard sounds/ }));
    expect(loadSoundboardEnabled()).toBe(true);
    expect(screen.getByRole("button", { name: "Airhorn" }).hasAttribute("disabled")).toBe(false);
  });

  it("keeps this browser's soundboard volume", async () => {
    vi.spyOn(soundboard, "catalog").mockResolvedValue(CATALOG);
    inCall();
    render(<SoundboardPanel />);
    await screen.findByRole("button", { name: "Airhorn" });

    fireEvent.change(screen.getByLabelText(/Soundboard volume/), {
      target: { value: "35" },
    });

    expect(loadSoundboardVolume()).toBe(35);
    expect(screen.getByText("35%")).toBeTruthy();
  });

  it("leaves the button up with sounds off, saying so", () => {
    setSoundboardEnabled(false);
    inCall();
    render(<SoundboardButton />);

    expect(screen.getByRole("button", { name: "Soundboard (sounds off)" })).toBeTruthy();
  });
});
