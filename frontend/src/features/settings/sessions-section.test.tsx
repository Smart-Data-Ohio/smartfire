import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { settings } from "../../sync/settings.ts";
import { SessionsSection } from "./sessions-section.tsx";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it("clears the badge before leaving after signing out this device", async () => {
  const clearAppBadge = vi.fn(async () => undefined);
  const assign = vi.fn();
  vi.stubGlobal("navigator", { clearAppBadge });
  vi.stubGlobal("window", { ...window, document, location: { assign } });
  vi.spyOn(settings, "sessions").mockResolvedValue({
    notice: null,
    sessions: [
      {
        id: 1,
        current: true,
        description: "This browser",
        ipAddress: null,
        createdAt: "2026-11-16T12:00:00Z",
        lastActiveAt: "2026-11-16T12:00:00Z",
      },
    ],
  });
  vi.spyOn(settings, "revokeSession").mockResolvedValue({ notice: null, sessions: [] });
  render(<SessionsSection />);
  fireEvent.click(await screen.findByRole("button", { name: "Sign out" }));
  fireEvent.click(
    within(screen.getByRole("alertdialog")).getByRole("button", { name: "Sign out" }),
  );
  await waitFor(() => expect(assign).toHaveBeenCalledWith("/app/session/new"));
  expect(clearAppBadge).toHaveBeenCalledOnce();
});
