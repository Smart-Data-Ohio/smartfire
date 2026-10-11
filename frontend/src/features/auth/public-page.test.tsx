import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import PUBLIC_PAGES from "../../../mock/s2/public-pages.json" with { type: "json" };
import { auth } from "../../sync/auth.ts";
import { installMockNetwork } from "../../test/mock-network.ts";
import { renderAuth, resetSignIn } from "./testing.tsx";

const network = installMockNetwork();

afterAll(() => network.restore());

afterEach(() => {
  vi.restoreAllMocks();
  document.getElementById("boot")?.remove();
  document.title = "";
});

/** The links in the landmark named `label`, as name and href. */
function links(label: string): readonly (readonly [string, string | null])[] {
  return within(screen.getByRole("navigation", { name: label }))
    .getAllByRole("link")
    .map((link) => [link.textContent ?? "", link.getAttribute("href")] as const);
}

describe("the public pages", () => {
  it.each([
    ["about", "About Smartfire", "Smartfire | About"],
    ["privacy", "Privacy Policy", "Smartfire | Privacy Policy"],
    ["terms", "Terms of Service", "Smartfire | Terms of Service"],
  ] as const)("draws /app/%s from the retained render", async (page, heading, title) => {
    await resetSignIn(network);
    await renderAuth(`/app/${page}`);

    expect(await screen.findByRole("heading", { level: 1, name: heading })).toBeTruthy();
    expect(document.title).toBe(title);
    expect(document.querySelector('meta[name="description"]')?.getAttribute("content")).toBe(
      PUBLIC_PAGES[page].description,
    );
    // The retained layout's header and footer, on the SPA's pages.
    expect(links("Public pages")).toEqual([
      ["About", "/app/about"],
      ["Privacy", "/app/privacy"],
      ["Terms", "/app/terms"],
      ["Sign in", "/app/session/new"],
    ]);
    expect(links("Footer")).toEqual([
      ["About", "/app/about"],
      ["Privacy Policy", "/app/privacy"],
      ["Terms of Service", "/app/terms"],
      ["Sign in", "/app/session/new"],
    ]);
    expect(screen.getByRole("link", { name: "Smartfire" }).getAttribute("target")).toBe("_blank");
  });

  it("shows who runs the workspace, as the policy configures it", async () => {
    await resetSignIn(network);
    await renderAuth("/app/about");

    const article = await screen.findByRole("article");

    // The operator name is escaped by the template and read back as text.
    expect(within(article).getByText("Example <&> Labs").tagName).toBe("STRONG");
    expect(
      within(article).getByRole("link", { name: "team+auth@example.test" }).getAttribute("href"),
    ).toBe("mailto:team%2Bauth@example.test");
    // In-text links stay on the SPA's pages, in a new tab as the retained page opens them.
    const privacy = within(article).getByRole("link", { name: "privacy policy" });

    expect(privacy.getAttribute("href")).toBe("/app/privacy");
    expect(privacy.getAttribute("target")).toBe("_blank");
    expect(
      within(article).getByRole("link", { name: "Sign in to this workspace" }).getAttribute("href"),
    ).toBe("/app/session/new");
  });

  it("dates the policies with the configured effective date", async () => {
    await resetSignIn(network);
    await renderAuth("/app/privacy");

    expect(await screen.findByText("Last updated: 2026-10-07")).toBeTruthy();
    expect(screen.getByText(/Contractors and guests, please note:/)).toBeTruthy();
  });

  it("is the same page signed out, under the signed-out boot", async () => {
    await resetSignIn(network);

    const boot = document.createElement("script");

    boot.id = "boot";
    boot.type = "application/json";
    boot.textContent = JSON.stringify({
      kind: "signedOut",
      workspace: { name: "Harbor", logoUrl: null, description: "" },
      signInMethods: { password: true, google: false, googleDomains: [] },
      firstRunPending: false,
      helpContact: null,
      version: "2.0.0",
      csrfToken: network.server.csrfToken(),
    });
    document.body.append(boot);
    await renderAuth("/app/terms");

    expect(await screen.findByRole("heading", { level: 1, name: "Terms of Service" })).toBeTruthy();
    // Nothing of the workspace's own: the retained pages name no workspace.
    expect(screen.queryByText("Harbor")).toBeNull();
  });

  it("offers to try again when the page can't be read", async () => {
    await resetSignIn(network);

    vi.spyOn(auth, "publicPage").mockRejectedValueOnce(new Error("Couldn't reach the server."));
    await renderAuth("/app/about");

    const alert = await screen.findByRole("alert");

    expect(alert.textContent).toContain("Couldn't reach the server.");

    expect(screen.queryByRole("article")).toBeNull();
    await userEvent.setup().click(within(alert).getByRole("button", { name: "Try again" }));
    await waitFor(() =>
      expect(screen.getByRole("heading", { level: 1, name: "About Smartfire" })).toBeTruthy(),
    );
  });
});
