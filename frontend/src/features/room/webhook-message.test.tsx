import { render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { messageFixture, userFixture } from "../../api/testing.ts";
import type { MessageDTO } from "../../store/model.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { hasCards } from "../cards/card-slot.tsx";
import { embedColor, WebhookEmbed } from "../cards/webhook-embed.tsx";
import { MessageRow } from "./message-row.tsx";

const AUTHOR = 500;

function webhookMessage(patch: Partial<MessageDTO> = {}): MessageDTO {
  return {
    ...messageFixture(41, 1, { creatorId: AUTHOR }),
    webhook: { name: "Deployments", username: null, avatarUrl: null },
    ...patch,
  };
}

function row(message: MessageDTO) {
  return render(
    <MessageRow message={message} groupStart mentionsMe={false} focused={false} live={false} />,
  );
}

beforeEach(() => {
  // jsdom has no matchMedia; a bot's generated avatar asks it for the theme.
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });
});

afterEach(() => {
  store.setState(initialState, true);
});

describe("a webhook's message", () => {
  it("shows the webhook's name and an APP tag rather than its author account", () => {
    mutations.mergeUsers([{ ...userFixture(AUTHOR, "Old name"), role: "bot" }]);
    row(webhookMessage());

    const header = screen.getByRole("banner");

    expect(within(header).getByText("Deployments")).toBeTruthy();
    expect(within(header).getByText("App")).toBeTruthy();
    expect(within(header).queryByText("Bot")).toBeNull();
    expect(screen.queryByText("Old name")).toBeNull();
    expect(screen.getByRole("article").getAttribute("aria-label")).toMatch(/^Deployments, /);
  });

  it("prefers the name it posted under, and its avatar", () => {
    row(
      webhookMessage({
        webhook: { name: "Deployments", username: "CI", avatarUrl: "/blobs/ci.png" },
      }),
    );

    expect(screen.getByText("CI")).toBeTruthy();
    expect(screen.queryByText("Deployments")).toBeNull();
    expect(screen.getByRole("article").querySelector('img[src="/blobs/ci.png"]')).not.toBeNull();
  });

  it("draws the webhook's initials when it has no picture and its account isn't loaded", () => {
    row(webhookMessage());

    const gutter = screen.getByRole("article").querySelector(".message-gutter");

    expect(gutter?.textContent).toBe("D");
  });

  it("leaves a person's message as it was", () => {
    mutations.mergeUsers([userFixture(AUTHOR, "Priya")]);
    row(messageFixture(42, 1, { creatorId: AUTHOR }));

    expect(screen.getByText("Priya")).toBeTruthy();
    expect(screen.queryByText("App")).toBeNull();
  });
});

describe("webhook embeds", () => {
  it("count as cards, so the slot loads for them", () => {
    expect(hasCards(messageFixture(1, 1))).toBe(false);
    expect(hasCards(webhookMessage({ embeds: [{ title: "Deployed" }] }))).toBe(true);
  });

  it("render as plain text with a linked title, fields and a footer", () => {
    render(
      <WebhookEmbed
        embed={{
          title: "Deploy <b>finished</b>",
          url: "https://ci.example.test/runs/9",
          description: "All green",
          color: 0x2ecc71,
          fields: [
            { name: "Branch", value: "main", inline: true },
            { name: "Commit", value: "abc123", inline: true },
          ],
          footer: { text: "CI" },
        }}
      />,
    );

    const title = screen.getByRole("link", { name: "Deploy <b>finished</b>" });

    expect(title.getAttribute("href")).toBe("https://ci.example.test/runs/9");
    expect(title.getAttribute("rel")).toContain("noopener");
    expect(screen.getByText("All green")).toBeTruthy();
    expect(screen.getByText("Branch")).toBeTruthy();
    expect(screen.getByText("abc123")).toBeTruthy();
    expect(screen.getByText("CI")).toBeTruthy();
    expect(document.querySelector("b")).toBeNull();
  });

  it("never links a title to anything but a web address", () => {
    render(<WebhookEmbed embed={{ title: "Sneaky", url: "javascript:alert(1)" }} />);

    expect(screen.queryByRole("link")).toBeNull();
    expect(screen.getByText("Sneaky")).toBeTruthy();
  });

  it("turns a 24-bit colour into a hex edge and ignores one out of range", () => {
    expect(embedColor(0x2ecc71)).toBe("#2ecc71");
    expect(embedColor(0)).toBe("#000000");
    expect(embedColor(0x1000000)).toBeUndefined();
    expect(embedColor(undefined)).toBeUndefined();
  });
});
