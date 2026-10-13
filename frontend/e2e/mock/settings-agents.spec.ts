import type { APIRequestContext, Page } from "@playwright/test";
import { expect, expectNoHorizontalOverflow, matrix, shot, type Theme, test } from "./support.ts";

/** The mock's password (mock/s2/settings.ts). */
const PASSWORD = "secret123456";

const SECRET = /^sfa_[0-9a-f]{64}$/;

/** Lapses the password confirmation, so creating an agent asks for it first. */
async function lapse(request: APIRequestContext): Promise<void> {
  const state = await (await request.get("/__mock/state")).json();

  await request.post("/__mock/lapse-sudo", {
    headers: { "X-CSRF-Token": state.csrfToken },
    data: { on: true, methods: ["password"] },
  });
}

/** Opens Settings → Agents with motion reduced, and waits for its list. */
async function openAgents(page: Page, theme: Theme = "light"): Promise<void> {
  await page.emulateMedia({ colorScheme: theme, reducedMotion: "reduce" });
  await page.goto("/app/settings/agents");
  await page.locator(".settings-page h1").waitFor({ state: "attached" });
  await expect(page.getByRole("button", { name: "Connect an agent" })).toBeVisible();
}

/** The copyable block labelled `label`. */
function block(page: Page, label: string) {
  return page.locator(".agents-code").filter({ has: page.getByText(label, { exact: true }) });
}

matrix("an agent's token shows once, then rotates and revokes", async ({ page, theme }) => {
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  await lapse(page.request);
  await openAgents(page, theme);
  await expect(page.getByText("No agents yet.", { exact: false })).toBeVisible();

  await page.getByRole("button", { name: "Connect an agent" }).click();

  // Without OAuth on the server, the web apps aren't offered.
  await expect(page.getByRole("radio", { name: /Claude Code/ })).toBeVisible();
  await expect(page.getByRole("radio", { name: /claude\.ai/ })).toHaveCount(0);
  await expect(page.getByRole("radio", { name: /ChatGPT/ })).toHaveCount(0);
  await expect(page.getByRole("radio", { name: /Grok/ })).toHaveCount(0);
  await expectNoHorizontalOverflow(page);
  await shot(page, "settings-agents-client", theme);

  await page.getByRole("radio", { name: /Claude Code/ }).check();
  await page.getByRole("button", { name: "Next" }).click();
  await page.getByRole("textbox", { name: "Name" }).fill("Helper");
  await page.getByRole("button", { name: "Next" }).click();
  await page.getByRole("checkbox", { name: "general" }).check();
  await expectNoHorizontalOverflow(page);
  await shot(page, "settings-agents-access", theme);
  await page.getByRole("button", { name: "Next" }).click();
  await page.getByRole("button", { name: "Create agent" }).click();

  // Issuing the token waits for the password, then goes through once.
  const confirm = page.getByRole("dialog", { name: "Confirm it's you" });

  await confirm.getByLabel("Password").fill(PASSWORD);
  await confirm.getByLabel("Password").press("Enter");
  await expect(confirm).toBeHidden();

  await expect(page.getByRole("heading", { name: "Connect Helper" })).toBeVisible();

  const secret = (await block(page, "Token").locator("pre").textContent()) ?? "";

  expect(secret).toMatch(SECRET);
  await expect(block(page, "Run in a terminal").locator("pre")).toContainText(
    "claude mcp add --transport http --scope user smartfire-helper",
  );
  await expect(block(page, "Run in a terminal").locator("pre")).toContainText(secret);

  await block(page, "Token").getByRole("button", { name: "Copy" }).click();
  await expect(page.getByText("Token copied")).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(secret);
  await expectNoHorizontalOverflow(page);
  await shot(page, "settings-agents-connect", theme);

  await page.getByRole("button", { name: "Done" }).click();
  await expect(page.getByRole("article", { name: "Helper" })).toBeVisible();

  // Once gone, the token is never shown again.
  await page.reload();

  const card = page.getByRole("article", { name: "Helper" });

  await expect(card).toBeVisible();
  await expect(card.getByText(`…${secret.slice(-4)}`)).toBeVisible();
  await expect(page.getByText(secret)).toHaveCount(0);
  await expectNoHorizontalOverflow(page);
  await shot(page, "settings-agents-card", theme);

  await card.getByRole("button", { name: "Rotate Primary token" }).click();

  const rotate = page.getByRole("dialog", { name: "Rotate Primary token" });

  await rotate.getByRole("button", { name: "Rotate token" }).click();

  const shown = page.getByRole("alertdialog", { name: "Helper's new token" });
  const rotated = (await shown.locator(".agents-code pre").first().textContent()) ?? "";

  expect(rotated).toMatch(SECRET);
  expect(rotated).not.toBe(secret);
  await shown.getByRole("button", { name: "Done" }).click();
  await expect(shown).toBeHidden();

  await expect(card.locator(".agents-token[data-state='revoked']")).toHaveCount(1);
  await card.getByRole("button", { name: "Revoke Primary token" }).click();
  await page
    .getByRole("alertdialog", { name: "Revoke Primary token?" })
    .getByRole("button", {
      name: "Revoke",
    })
    .click();
  await expect(page.getByText("Primary token was revoked")).toBeVisible();
  await expect(card.locator(".agents-token[data-state='revoked']")).toHaveCount(2);
  await expect(card.getByRole("button", { name: /^Revoke/ })).toHaveCount(0);
});

test("an administrator turns personal agents off, and Settings → Agents explains", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/app/admin/workspace");

  const toggle = page.getByRole("switch", { name: /Members can connect their own agents/ });

  await expect(toggle).toBeChecked();
  await toggle.click();
  await expect(page.getByText("Personal agents off")).toBeVisible();

  await page.getByRole("spinbutton", { name: "Agents per member" }).fill("1");
  await page.getByRole("button", { name: "Save limit" }).click();
  await expect(page.getByText("Agent limit saved")).toBeVisible();

  await page.goto("/app/settings/agents");
  await expect(page.getByText(/Personal agents are turned off/)).toBeVisible();
  await expect(page.getByRole("button", { name: "Connect an agent" })).toHaveCount(0);
});
