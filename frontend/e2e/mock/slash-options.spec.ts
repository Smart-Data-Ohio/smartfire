import type { Page } from "@playwright/test";
import {
  DESKTOP,
  expect,
  expectNoHorizontalOverflow,
  expectTouchTargets,
  PHONE_TOUCH,
  ROOM_IDS,
  test,
} from "./support.ts";

const ENGINEERING = `r/${ROOM_IDS.engineering}`;

async function openRoom(page: Page): Promise<void> {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/${ENGINEERING}`);
  await composer(page).waitFor();
}

function composer(page: Page) {
  return page.getByRole("textbox", { name: "Message #engineering" });
}

function options(page: Page) {
  return page.getByRole("group", { name: "Options for /deploy" });
}

function field(page: Page, name: string) {
  return options(page).getByRole(name === "environment" ? "combobox" : "textbox", { name });
}

/**
 * /deploy from the menu, its subcommand, then its options: a client-side error and its fix, an
 * optional option out of range and its fix, a 422 from the server on a field and its fix, then
 * the send. On a phone it also holds the fields to finger-sized targets and no sideways scroll.
 */
async function deployFlow(page: Page, phone: boolean): Promise<void> {
  await openRoom(page);
  await composer(page).click();
  await composer(page).pressSequentially("/dep");
  await expect(page.getByRole("option", { name: /\/deploy <run>/ })).toBeVisible();
  await composer(page).press("Enter");

  const subcommand = options(page).getByRole("combobox", { name: "subcommand" });

  await expect(subcommand).toBeFocused();
  await expect(page.getByRole("listbox", { name: "Subcommands" })).toBeVisible();
  await subcommand.press("Enter");
  await expect(field(page, "service")).toBeFocused();

  const send = page.getByRole("button", { name: "Send message" });

  await expect(send).toBeDisabled();

  // Too short: the error shows once the field is left.
  await field(page, "service").pressSequentially("x");
  await field(page, "service").press("Tab");
  await expect(field(page, "environment")).toBeFocused();
  await expect(options(page).getByText("Must be 2 to 40 characters.")).toBeVisible();
  await expect(field(page, "service")).toHaveAttribute("aria-invalid", "true");

  // A choice comes from its pick list.
  await expect(page.getByRole("listbox", { name: "Choices" })).toBeVisible();

  if (phone) {
    await expectTouchTargets(page, ".command-options .autocomplete");
    await expectNoHorizontalOverflow(page);
  }

  await field(page, "environment").press("ArrowDown");
  await field(page, "environment").press("Enter");
  await expect(field(page, "environment")).toHaveValue("Production");
  await expect(send).toBeDisabled();

  // Fixed.
  await field(page, "service").fill("api");
  await expect(options(page).getByText("Must be 2 to 40 characters.")).toBeHidden();
  await expect(field(page, "service")).not.toHaveAttribute("aria-invalid", "true");
  await expect(send).toBeEnabled();

  // An optional option, out of range, then in range.
  await options(page).getByRole("button", { name: "Add replicas" }).click();
  await expect(field(page, "replicas")).toBeFocused();
  await field(page, "replicas").pressSequentially("42");
  await expect(send).toBeDisabled();
  await field(page, "replicas").press("Enter");
  await expect(options(page).getByText("Must be 1 to 10.")).toBeVisible();

  if (phone) {
    await expectTouchTargets(page, ".command-options");
    await expectNoHorizontalOverflow(page);
  }

  await field(page, "replicas").fill("3");
  await expect(send).toBeEnabled();

  // The server refuses a value the client let through: its error lands on that field. The send
  // goes out once with a service too short, so the 422 is the mock server's own.
  await page.route(
    "**/api/v1/rooms/*/slash_commands",
    async (route) => {
      const body = route.request().postDataJSON();

      await route.continue({
        postData: {
          ...body,
          options: { ...body.options, values: { ...body.options.values, service: "a" } },
        },
      });
    },
    { times: 1 },
  );
  await send.click();
  await expect(options(page).getByText("Length is out of range.")).toBeVisible();
  await expect(field(page, "service")).toHaveAttribute("aria-invalid", "true");
  await expect(field(page, "service")).toBeFocused();
  await expect(field(page, "replicas")).toHaveValue("3");

  await field(page, "service").fill("api-gateway");
  await expect(options(page).getByText("Length is out of range.")).toBeHidden();

  const posted = page.waitForRequest(
    (request) => request.method() === "POST" && request.url().endsWith("/slash_commands"),
  );

  await send.click();
  expect((await posted).postDataJSON()).toEqual({
    text: "/deploy run service:api-gateway environment:production replicas:3",
    threadId: null,
    options: {
      subcommand: "run",
      values: { service: "api-gateway", environment: "production", replicas: 3 },
    },
  });
  await expect(page.getByText("Sent to Ember")).toBeVisible();
  await expect(options(page)).toBeHidden();
  await expect(composer(page)).toBeFocused();
}

test.describe("typed slash command options", () => {
  test("pick a command, fill its options, fix the errors and send (desktop)", async ({ page }) => {
    await page.setViewportSize(DESKTOP);
    await deployFlow(page, false);
  });

  test("Escape turns the fields back into plain text", async ({ page }) => {
    await page.setViewportSize(DESKTOP);
    await openRoom(page);
    await composer(page).click();
    await composer(page).pressSequentially("/deploy");
    await composer(page).press("Enter");
    await options(page).getByRole("combobox", { name: "subcommand" }).press("Enter");
    await field(page, "service").pressSequentially("api");
    await field(page, "service").press("Escape");
    await expect(options(page)).toBeHidden();
    await expect(composer(page)).toHaveValue("/deploy run service:api ");
    await expect(composer(page)).toBeFocused();
  });
});

test.describe("typed slash command options on a touch phone", () => {
  test.use(PHONE_TOUCH);

  test("pick a command, fill its options, fix the errors and send (phone)", async ({ page }) => {
    await deployFlow(page, true);
  });
});
