import type { APIRequestContext, Page } from "@playwright/test";
import { CARD_IDS } from "../../mock/s3/cards.ts";
import { expect, test } from "./support.ts";

const ROOM = CARD_IDS.room;

const { messages } = CARD_IDS;

const OPEN = "Pull request: Rate limit the sync endpoint with a token bucket";

async function control(
  request: APIRequestContext,
  data: Readonly<Record<string, number | string>>,
) {
  const state = await (await request.get("/__mock/state")).json();

  return request.post("/__mock/cards", { headers: { "X-CSRF-Token": state.csrfToken }, data });
}

/** The write API 404s until Discuss has mapped a discussion thread in the room. */
async function discuss(request: APIRequestContext) {
  const state = await (await request.get("/__mock/state")).json();

  const response = await request.post(
    `/api/v1/rooms/${ROOM}/github/pull_requests/${CARD_IDS.pullRequests.open}/discussion`,
    {
      headers: { "X-CSRF-Token": state.csrfToken },
      data: { messageId: messages.githubOpen },
    },
  );

  expect(response.ok()).toBeTruthy();
}

async function openCard(page: Page) {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`/app/r/${ROOM}/m/${messages.githubOpen}`);
  await page.getByRole("main").waitFor();

  const card = page.getByRole("region", { name: OPEN });

  await expect(card).toBeVisible();

  return card;
}

test("shows the write actions after Discuss on a fresh pull request", async ({ page }) => {
  const card = await openCard(page);

  await expect(card.getByRole("button", { name: "Comment" })).toHaveCount(0);
  await card.getByRole("button", { name: "Discuss" }).click();
  await expect(page.getByRole("button", { name: "Comment" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Review", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Request reviewers" })).toBeVisible();
});

test("comments, reviews and requests reviewers from the pull request card", async ({
  page,
  request,
}) => {
  await discuss(request);

  const card = await openCard(page);

  await expect(card.getByRole("button", { name: "Review", exact: true })).toBeVisible();
  await expect(card.getByRole("button", { name: "Request reviewers" })).toBeVisible();

  await card.getByRole("button", { name: "Comment" }).click();
  await expect(card.getByRole("alert")).toHaveText("Write a comment first.");

  await card.getByRole("textbox", { name: "Comment" }).fill("Looks good.");
  await card.getByRole("button", { name: "Comment" }).click();
  await expect(page.getByText("Comment posted on GitHub as @maya.").first()).toBeVisible();

  await card.getByRole("button", { name: "Review", exact: true }).click();
  await page.getByRole("menuitem", { name: "Request changes" }).click();

  const dialog = page.getByRole("dialog", { name: "Request changes" });

  await dialog.getByRole("button", { name: "Request changes" }).click();
  await expect(dialog.getByRole("alert")).toHaveText(
    "Add a note describing the requested changes.",
  );

  await dialog.getByRole("textbox", { name: "Note" }).fill("Rename the limiter.");
  await dialog.getByRole("button", { name: "Request changes" }).click();
  await expect(card.getByText("Changes requested")).toBeVisible();

  await card.getByRole("button", { name: "Request reviewers" }).click();

  const reviewers = page.getByRole("dialog", { name: "Request reviewers" });

  await reviewers.getByRole("textbox", { name: "Reviewers" }).fill("alice, @bob");
  await reviewers.getByRole("button", { name: "Request reviewers" }).click();
  await expect(
    page.getByText("Requested review from @alice, @bob on GitHub as @maya.").first(),
  ).toBeVisible();
});

test("hides the write actions when the GitHub account can't be used", async ({ page, request }) => {
  await discuss(request);
  await control(request, { op: "github-writes", enabled: 0 });

  const pending = page.waitForResponse((response) => response.url().includes("/actions"));
  const card = await openCard(page);

  await pending;
  await expect(card.getByText("Open", { exact: true })).toBeVisible();
  await expect(card.getByRole("button", { name: "Comment" })).toHaveCount(0);
  await expect(card.getByRole("button", { name: "Review", exact: true })).toHaveCount(0);
  await expect(card.getByRole("button", { name: "Request reviewers" })).toHaveCount(0);
});
