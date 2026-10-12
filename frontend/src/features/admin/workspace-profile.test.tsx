import { fireEvent, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { onboardingMockupPng } from "../../../mock/s2/assets.ts";
import type { Workspace } from "../../gen/Workspace.ts";
import { admin } from "../../sync/admin.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { resetSignIn } from "../auth/testing.tsx";
import { AdminContext } from "./admin-parts.tsx";
import { renderInRouter } from "./testing.tsx";
import { WorkspaceProfile } from "./workspace-profile.tsx";

let network: MockNetwork;

afterEach(() => {
  network.restore();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

beforeEach(async () => {
  network = installMockNetwork();
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );
  URL.createObjectURL = () => "blob:preview";
  URL.revokeObjectURL = () => undefined;
  await resetSignIn(network);
});

function Profile({ initial }: { readonly initial: Workspace }) {
  const [workspace, replace] = useState(initial);

  return (
    <AdminContext value={{ workspace, replace }}>
      <WorkspaceProfile />
    </AdminContext>
  );
}

const file = () =>
  new File([Uint8Array.from(onboardingMockupPng())], "splash.png", { type: "image/png" });

it("uploads, previews, replaces and removes the invite splash through the shared slot", async () => {
  await renderInRouter(<Profile initial={await admin.workspace()} />);
  expect(screen.getByRole("button", { name: "Upload invite splash" })).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Choose invite splash image"), {
    target: { files: [file()] },
  });
  await screen.findByRole("button", { name: "Replace invite splash" });
  const first = (await admin.workspace()).inviteSplashUrl;
  expect(first).toMatch(/\/blobs\//);
  expect(screen.getByAltText("Current invite splash").getAttribute("src")).toBe(first);
  fireEvent.change(screen.getByLabelText("Choose invite splash image"), {
    target: { files: [file()] },
  });
  await waitFor(async () => expect((await admin.workspace()).inviteSplashUrl).not.toBe(first));
  fireEvent.click(screen.getByRole("button", { name: "Remove invite splash" }));
  await screen.findByRole("button", { name: "Upload invite splash" });
  expect((await admin.workspace()).inviteSplashUrl).toBeNull();
});

it("refuses the splash format and size before uploading", async () => {
  await renderInRouter(<Profile initial={await admin.workspace()} />);
  fireEvent.change(screen.getByLabelText("Choose invite splash image"), {
    target: { files: [new File(["svg"], "splash.svg", { type: "image/svg+xml" })] },
  });
  expect(screen.getByRole("alert").textContent).toContain("Choose a PNG, JPEG, GIF or WebP image.");
  fireEvent.change(screen.getByLabelText("Choose invite splash image"), {
    target: {
      files: [new File([new Uint8Array(10 * 1024 * 1024 + 1)], "big.png", { type: "image/png" })],
    },
  });
  expect(screen.getByRole("alert").textContent).toContain("Choose an image of 10 MB or less.");
  expect((await admin.workspace()).inviteSplashUrl).toBeNull();
});

it("keeps splash editing controls out of the member view", async () => {
  await renderInRouter(
    <Profile initial={{ ...(await admin.workspace()), canAdminister: false }} />,
  );
  expect(screen.queryByLabelText("Choose invite splash image")).toBeNull();
  expect(screen.queryByRole("button", { name: "Upload invite splash" })).toBeNull();
});
