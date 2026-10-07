import type { Locator, Page } from "@playwright/test";
import {
  expect,
  openApp,
  postMessage,
  ROOM_IDS,
  shot,
  syncWelcomed,
  type Theme,
  test,
  USER_IDS,
} from "./support.ts";

/**
 * Unread against notifications in the sidebar, as Discord and Slack draw them: a plain message
 * makes a row bold and bright with the nub on the left edge, and only a notification (a mention,
 * a DM, a room set to notify for everything) puts a red count on it.
 */

const sidebar = (page: Page) => page.getByRole("complementary", { name: "Conversations" });

/** A conversation's row link, by the name it shows. */
function row(page: Page, name: string): Locator {
  return sidebar(page)
    .locator(".sidebar-row")
    .filter({ has: page.locator(".sidebar-row-name", { hasText: new RegExp(`^${name}$`) }) })
    .first();
}

/** The weight and colour of a row's name, and how far its nub has grown in. */
function looks(target: Locator) {
  return target.evaluate((element) => {
    const style = getComputedStyle(element);
    const nub = getComputedStyle(element, "::before");

    return {
      weight: Number(style.fontWeight),
      color: style.color,
      nub: Number(nub.opacity),
    };
  });
}

/** The computed colour `color` resolves to, to compare against a row's. */
function resolved(page: Page, color: string): Promise<string> {
  return page.evaluate((value) => {
    const probe = document.createElement("span");

    probe.style.color = value;
    document.body.append(probe);

    const result = getComputedStyle(probe).color;

    probe.remove();

    return result;
  }, color);
}

for (const theme of ["light", "dark"] as const satisfies readonly Theme[]) {
  test(`a plain message makes a row bold with no count; a mention adds a red one (${theme})`, async ({
    page,
    request,
  }) => {
    const welcomed = syncWelcomed(page);

    await openApp(page, `r/${ROOM_IDS.quiet}`, theme);
    await welcomed;

    const engineering = row(page, "engineering");
    const badge = engineering.locator(".badge");

    await expect(engineering).not.toHaveAttribute("data-state", "unread");

    const read = await looks(engineering);

    await postMessage(request, {
      roomId: ROOM_IDS.engineering,
      userId: USER_IDS.jonah,
      markdown: "The deploy went out.",
    });

    await expect(engineering).toHaveAttribute("data-state", "unread");
    await expect.poll(async () => (await looks(engineering)).nub).toBe(1);

    const unread = await looks(engineering);

    expect(unread.weight).toBeGreaterThan(read.weight);
    expect(unread.color).not.toBe(read.color);
    // White in the dark theme, the ink at full strength in the light one.
    expect(unread.color).toBe(
      await resolved(page, theme === "dark" ? "oklch(100% 0 0)" : "var(--text)"),
    );
    // Plain unread: no count.
    await expect(badge).toHaveAttribute("data-open", "false");

    await postMessage(request, {
      roomId: ROOM_IDS.engineering,
      userId: USER_IDS.jonah,
      markdown: "@[Riel St. Amand] can you check the logs?",
    });

    await expect(badge).toHaveAttribute("data-open", "true");
    await expect(badge).toHaveAttribute("data-tone", "danger");
    await expect(engineering).toHaveAccessibleName("engineering 1 mentions");
    await expect(engineering).toHaveAttribute("data-state", "unread");
    await shot(page, "unread-row", theme);
  });

  test(`a folded category carries the nub, and a red count only for its mentions (${theme})`, async ({
    page,
    request,
  }) => {
    const welcomed = syncWelcomed(page);

    await openApp(page, `r/${ROOM_IDS.quiet}`, theme);
    await welcomed;

    // "Team" holds #announcements, read. Fold it, then a plain message arrives there.
    const team = page.locator('[data-drop-section="category-2"]');
    const teamBadge = team.locator(".sidebar-section-heading > .badge");

    await team.locator(".sidebar-section-trigger").click();
    await expect(team).toHaveAttribute("data-open", "false");
    await expect(team).not.toHaveAttribute("data-unread");

    // Notify for mentions only, so the plain message is unread but no notification.
    await page.evaluate(async (roomId) => {
      const token = document.querySelector<HTMLMetaElement>('meta[name="csrf-token"]')?.content;

      await fetch(`/api/v1/rooms/${roomId}/involvement`, {
        method: "PUT",
        headers: { "Content-Type": "application/json", "X-CSRF-Token": token ?? "" },
        body: JSON.stringify({ involvement: "mentions" }),
      });
    }, ROOM_IDS.announcements);

    await postMessage(request, {
      roomId: ROOM_IDS.announcements,
      userId: USER_IDS.maya,
      markdown: "Office closed Friday.",
    });

    await expect(team).toHaveAttribute("data-unread", "true");
    await expect(teamBadge).toHaveAttribute("data-open", "false");
    await expect
      .poll(async () => (await looks(team.locator(".sidebar-section-trigger"))).nub)
      .toBe(1);
    // The unread room still peeks out below the folded heading.
    await expect(row(page, "announcements")).toHaveAttribute("data-state", "unread");

    // "Launch" holds #design (plain unread) and #launch-planning (one unread mention).
    const launch = page.locator('[data-drop-section="category-1"]');
    const launchBadge = launch.locator(".sidebar-section-heading > .badge");

    await expect(launchBadge).toHaveAttribute("data-open", "false");
    await launch.locator(".sidebar-section-trigger").click();
    await expect(launch).toHaveAttribute("data-unread", "true");
    await expect(launchBadge).toHaveAttribute("data-open", "true");
    await expect(launchBadge.locator(":scope > .visually-hidden")).toHaveText("1 notification");
    await shot(page, "unread-folded", theme);

    // Open again, the heading goes back to plain and the rows carry their own state.
    await launch.locator(".sidebar-section-trigger").click();
    await expect(launch).not.toHaveAttribute("data-unread");
    await expect(launchBadge).toHaveAttribute("data-open", "false");
  });
}
