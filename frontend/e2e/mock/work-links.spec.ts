import { S4_WORK_IDS } from "../../mock/s4/seed.ts";
import { BOARD_POST_IDS, BOARD_ROOM_ID } from "../../mock/s6/seed.ts";
import {
  DESKTOP,
  expect,
  openApp,
  PHONE,
  PHONE_TOUCH,
  ROOM_IDS,
  SHOTS,
  shot,
  test,
} from "./support.ts";

const POST = BOARD_POST_IDS.apiPagination;

// Screenshots only: post-links.test.tsx covers the form, mock/s6/work-links.test.ts the server's
// words, and the work thread test below the bare links URL and a real link round trip.
if (SHOTS) {
  test("a post links a pull request, refuses a bad URL in the server's words, and unlinks it", async ({
    page,
  }) => {
    await page.setViewportSize(DESKTOP);
    await openApp(page, `r/${BOARD_ROOM_ID}/t/${POST}`);

    const pane = page.locator("aside.right-pane");
    const linked = pane.getByRole("region", { name: "Linked" });

    await expect(linked).toBeVisible();
    await linked.getByRole("button", { name: "Link", exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD_ROOM_ID}/t/${POST}/links$`));

    const form = linked.getByRole("form", { name: "Link to this work" });

    await form.getByLabel("Pull request URL").fill("https://example.com/not-a-pr");
    await form.getByRole("button", { name: "Link pull request" }).click();
    await expect(
      form.getByText(
        "Enter a GitHub pull request URL, like https://github.com/owner/repo/pull/123.",
      ),
    ).toBeVisible();
    await expect(form.getByLabel("Pull request URL")).toHaveAttribute("aria-invalid", "true");

    await form.getByLabel("Pull request URL").fill("https://github.com/acme/api/pull/77");
    await shot(page, "work-links-form", "light");
    await form.getByRole("button", { name: "Link pull request" }).click();

    await expect(form).toHaveCount(0);
    await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD_ROOM_ID}/t/${POST}$`));
    await expect(linked.getByRole("link", { name: "acme/api#77" })).toBeVisible();
    await page.mouse.move(0, 0);
    await shot(page, "work-links", "light");

    await linked.getByRole("button", { name: "Remove link acme/api#77" }).click();
    await expect(linked.getByRole("link", { name: "acme/api#77" })).toHaveCount(0);
  });

  test("the classic links URL opens the link form on its post", async ({ page }) => {
    await page.setViewportSize(PHONE);
    await openApp(page, `t/${POST}/links`, "dark");

    await expect(page).toHaveURL(new RegExp(`/app/r/${BOARD_ROOM_ID}/t/${POST}/links$`));
    await expect(page.getByRole("form", { name: "Link to this work" })).toBeVisible();
    await shot(page, "work-links-form", "dark");
  });
}

test("a work thread's classic links URL opens the link editor", async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  await openApp(page, `t/${S4_WORK_IDS.agentOwned}/links`);

  const thread = `/app/r/${ROOM_IDS.general}/t/${S4_WORK_IDS.agentOwned}`;

  await expect(page).toHaveURL(new RegExp(`${thread}/links$`));

  const work = page.locator("aside.right-pane").getByRole("region", { name: "Work" });
  const form = work.getByRole("form", { name: "Link to this work" });

  await expect(form).toBeVisible();
  await form.getByLabel("Pull request URL").fill("https://github.com/acme/api/pull/88");
  await form.getByRole("button", { name: "Link pull request" }).click();

  await expect(form).toHaveCount(0);
  await expect(page).toHaveURL(new RegExp(`${thread}$`));
  await expect(work.getByRole("link", { name: /acme\/api#88/ })).toBeVisible();
});

test("a desktop work thread keeps cancel and submit on screen when it has many links", async ({
  page,
  request,
}) => {
  const threadId = S4_WORK_IDS.agentOwned;
  const state = await (await request.get("/__mock/state")).json();

  for (let index = 0; index < 16; index += 1) {
    const response = await request.post(`/api/v1/threads/${threadId}/work/links`, {
      headers: { "X-CSRF-Token": state.csrfToken },
      data: {
        kind: "pull_request",
        pullRequestUrl: `https://github.com/acme/api/pull/${400 + index}`,
      },
    });

    expect(response.ok()).toBe(true);
  }

  await page.setViewportSize({ width: 1280, height: 720 });
  await openApp(page, `r/${ROOM_IDS.general}/t/${threadId}`);

  const work = page.locator("aside.right-pane").getByRole("region", { name: "Work" });

  await work.getByRole("button", { name: "Link", exact: true }).click();

  const form = work.getByRole("form", { name: "Link to this work" });
  const submit = form.getByRole("button", { name: "Link pull request" });
  const cancel = form.getByRole("button", { name: "Cancel" });

  await expect(submit).toBeInViewport({ ratio: 1 });
  await expect(cancel).toBeInViewport({ ratio: 1 });
  await expect(submit).toBeEnabled();
  await expect(cancel).toBeEnabled();
  await submit.click({ trial: true });
  await cancel.click({ trial: true });

  await form.getByLabel("Pull request URL").fill("https://github.com/acme/api/pull/499");
  await expect(submit).toBeInViewport({ ratio: 1 });
  await expect(cancel).toBeInViewport({ ratio: 1 });

  await page.setViewportSize({ width: 1280, height: 600 });
  await expect(submit).toBeInViewport({ ratio: 1 });
  await expect(cancel).toBeInViewport({ ratio: 1 });
  await submit.click({ trial: true });
  await cancel.click({ trial: true });
  await submit.click();
  await expect(form).toHaveCount(0);
});

test("a desktop board post keeps cancel and submit on screen when it has many links", async ({
  page,
  request,
}) => {
  const state = await (await request.get("/__mock/state")).json();

  for (let index = 0; index < 16; index += 1) {
    const response = await request.post(`/api/v1/threads/${POST}/work/links`, {
      headers: { "X-CSRF-Token": state.csrfToken },
      data: {
        kind: "pull_request",
        pullRequestUrl: `https://github.com/acme/api/pull/${500 + index}`,
      },
    });

    expect(response.ok()).toBe(true);
  }

  await page.setViewportSize({ width: 1280, height: 600 });
  await openApp(page, `r/${BOARD_ROOM_ID}/t/${POST}`);

  const linked = page.locator("aside.right-pane").getByRole("region", { name: "Linked" });

  await linked.getByRole("button", { name: "Link", exact: true }).click();

  const form = linked.getByRole("form", { name: "Link to this work" });
  const submit = form.getByRole("button", { name: "Link pull request" });
  const cancel = form.getByRole("button", { name: "Cancel" });

  await expect(submit).toBeInViewport({ ratio: 1 });
  await expect(cancel).toBeInViewport({ ratio: 1 });
  await expect(submit).toBeEnabled();
  await expect(cancel).toBeEnabled();
  await submit.click({ trial: true });
  await cancel.click({ trial: true });
});

test.describe("a phone's links editor", () => {
  test.use(PHONE_TOUCH);

  test("keeps submit on screen when the thread has many links", async ({ page, request }) => {
    const threadId = S4_WORK_IDS.agentOwned;
    const state = await (await request.get("/__mock/state")).json();

    for (let index = 0; index < 16; index += 1) {
      const response = await request.post(`/api/v1/threads/${threadId}/work/links`, {
        headers: { "X-CSRF-Token": state.csrfToken },
        data: {
          kind: "pull_request",
          pullRequestUrl: `https://github.com/acme/api/pull/${300 + index}`,
        },
      });

      expect(response.ok()).toBe(true);
    }

    await openApp(page, `r/${ROOM_IDS.general}/t/${threadId}`);

    const work = page.locator("aside.right-pane").getByRole("region", { name: "Work" });

    await work.getByRole("button", { name: "Link", exact: true }).click();

    const form = work.getByRole("form", { name: "Link to this work" });
    const submit = form.getByRole("button", { name: "Link pull request" });

    await expect(submit).toBeInViewport({ ratio: 1 });
    await expect(submit).toBeEnabled();
    await form.getByLabel("Pull request URL").fill("https://github.com/acme/api/pull/399");
    await expect(submit).toBeInViewport({ ratio: 1 });
    await submit.click();
    await expect(form).toHaveCount(0);
  });
});
