import type { APIRequestContext, Page } from "@playwright/test";
import { expect, expectNoHorizontalOverflow, matrix, shot, test } from "./support.ts";

/** The mock's password (mock/s2/settings.ts). */
const PASSWORD = "secret123456";

/** Where the mock's example app answers (mock/s2/oauth.ts). */
const CALLBACK = "https://client.example/callback";

async function csrf(request: APIRequestContext): Promise<string> {
  return (await (await request.get("/__mock/state")).json()).csrfToken;
}

/** Makes an agent named Helper in #general that can read, as Settings would. */
async function makeHelper(request: APIRequestContext): Promise<void> {
  const answer = await request.post("/api/v1/me/agents", {
    headers: { "X-CSRF-Token": await csrf(request) },
    data: {
      name: "Helper",
      iconName: "owl",
      provider: null,
      runtime: "codex",
      roomIds: [1],
      capabilities: ["read_messages"],
      tokenName: null,
      expiresAt: null,
    },
  });

  expect(answer.status()).toBe(201);
}

/** Turns OAuth on and makes the request an app's authorize call would; its approval page. */
async function authorize(request: APIRequestContext): Promise<string> {
  const token = await csrf(request);

  await request.post("/__mock/oauth-available", {
    headers: { "X-CSRF-Token": token },
    data: { on: true },
  });

  const made = await request.post("/__mock/oauth-authorization", {
    headers: { "X-CSRF-Token": token },
    data: { state: "xyz" },
  });

  return (await made.json()).path;
}

/** Lapses the password confirmation, so approving asks for it first. */
async function lapse(request: APIRequestContext): Promise<void> {
  await request.post("/__mock/lapse-sudo", {
    headers: { "X-CSRF-Token": await csrf(request) },
    data: { on: true, methods: ["password"] },
  });
}

/** Stands in for the app: its callback page, so the browser has somewhere to land. */
async function serveCallback(page: Page): Promise<void> {
  await page.route(`${CALLBACK}*`, (route) =>
    route.fulfill({ contentType: "text/html", body: "<h1>Back in the app</h1>" }),
  );
}

/** Opens the approval page with motion reduced, and waits for the request. */
async function open(page: Page, path: string, theme: "light" | "dark"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto(path);
  await expect(
    page.getByRole("heading", { name: "Example Assistant wants to connect as one of your agents" }),
  ).toBeVisible();
}

matrix("approving an app as an existing agent shows what changes", async ({ page, theme }) => {
  await makeHelper(page.request);
  const path = await authorize(page.request);

  await serveCallback(page);
  await lapse(page.request);
  await open(page, path, theme);

  await expect(page.getByText("client.example", { exact: true })).toBeVisible();
  await expect(page.getByText(/hasn't verified this app/)).toBeVisible();
  // No connection yet, so a new agent is picked; Helper is offered.
  await expect(page.getByRole("radio", { name: /A new agent/ })).toBeChecked();
  await page.getByRole("radio", { name: /Helper/ }).check();

  const diff = page.getByRole("region", { name: "Changes to Helper's access" });

  await expect(diff).toContainText("No change");
  await page.getByRole("checkbox", { name: "design" }).check();
  await page.getByRole("checkbox", { name: /React/ }).check();
  // Room by room: design is new, and general (where it only read) gains reacting.
  await expect(diff.getByRole("listitem").filter({ hasText: "Joins" })).toHaveText(
    "Joins design: can read messages, react",
  );
  await expect(diff.getByRole("listitem").filter({ hasText: /^In / })).toHaveText(
    "In general: can now react",
  );
  await expectNoHorizontalOverflow(page);
  await shot(page, "oauth-consent-diff", theme);

  await page.getByRole("button", { name: "Approve" }).click();

  const confirm = page.getByRole("dialog", { name: "Confirm it's you" });

  await confirm.getByLabel("Password").fill(PASSWORD);
  await confirm.getByLabel("Password").press("Enter");

  await expect(page.getByRole("heading", { name: "Back in the app" })).toBeVisible();

  const landed = new URL(page.url());

  expect(landed.origin + landed.pathname).toBe(CALLBACK);
  expect(landed.searchParams.get("state")).toBe("xyz");
  expect(landed.searchParams.get("code")).toMatch(/^mock_code_/);

  // Settings lists the connection on Helper's card, with its client and dates.
  await page.goto("/app/settings/agents");

  const connections = page.getByRole("region", { name: "Helper's connections" });

  await expect(connections.getByText("Example Assistant")).toBeVisible();
  await expect(connections.getByText(/^Connected .* · expires .* · never used$/)).toBeVisible();
  await expect(connections.getByRole("button", { name: "Revoke Example Assistant" })).toBeVisible();

  // The used request is gone: opening it again says so.
  await page.goto(path);
  await expect(page.getByRole("heading", { name: "This request has ended" })).toBeVisible();
});

matrix("denying an app sends it back with its refusal", async ({ page, theme }) => {
  await makeHelper(page.request);
  const path = await authorize(page.request);

  await serveCallback(page);
  await open(page, path, theme);
  await page.getByRole("radio", { name: /Helper/ }).check();
  await page.getByRole("checkbox", { name: "general" }).uncheck();

  const diff = page.getByRole("region", { name: "Changes to Helper's access" });

  await expect(diff.getByRole("listitem").filter({ hasText: "Leaves" })).toHaveText(
    "Leaves general: can no longer read messages",
  );
  await expect(diff.getByRole("listitem")).toHaveCount(1);
  await expectNoHorizontalOverflow(page);
  await shot(page, "oauth-consent-deny", theme);

  await page.getByRole("button", { name: "Deny" }).click();
  await expect(page.getByRole("heading", { name: "Back in the app" })).toBeVisible();

  const landed = new URL(page.url());

  expect(landed.searchParams.get("error")).toBe("access_denied");
  expect(landed.searchParams.get("state")).toBe("xyz");

  // Nothing changed for Helper.
  await page.goto("/app/settings/agents");
  await expect(page.getByRole("article", { name: "Helper" })).toContainText("general");
  await expect(page.getByRole("region", { name: "Helper's connections" })).toHaveCount(0);
});

test("the error page explains a request Smartfire couldn't take", async ({ page }) => {
  await page.goto("/app/oauth/error?error=invalid_redirect_uri");

  await expect(
    page.getByRole("heading", { name: "The app's return address doesn't match" }),
  ).toBeVisible();
  await expect(page.getByText("invalid_redirect_uri")).toBeVisible();
  await page.getByRole("link", { name: "Back to Smartfire" }).click();
  await expect(page).toHaveURL(/\/app\/$/);
});
