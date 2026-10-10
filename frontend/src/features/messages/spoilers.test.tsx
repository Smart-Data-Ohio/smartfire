import { readFileSync } from "node:fs";
import { join } from "node:path";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { inlineMentions } from "../../lib/body-html.ts";
import { workDetailFixture } from "../work/test-fixtures.ts";
import { WorkResult } from "../work/work-details.tsx";
import { BodyHtml } from "./body-html.tsx";
import { SPOILER_LABEL, useSpoilerReveal } from "./spoilers.ts";

/** A spoiler span as crates/richtext renders it. */
const spoiler = (inner: string) => `<span class="spoiler" data-spoiler="">${inner}</span>`;

/** `||@[David] is the killer :party:||`: a mention chip and a custom emoji image inside. */
const MENTION_AND_EMOJI = `<p>${spoiler(
  '<span class="mention mention--user-1"><a class="btn avatar" href="/users/1">D</a>' +
    '<button class="profile-card-name" type="button">David</button></span> is the killer ' +
    '<img class="icon" src="/icons/party.png" alt=":party:"> <a href="https://example.com">clue</a>',
)}</p>`;

/**
 * room.css as the page has it. jsdom's cascade ignores rules inside `@layer`, so the one layer
 * wrapper comes off. Everything in the file is in that layer, so the order is unchanged.
 */
function addRoomStyles(): HTMLStyleElement {
  const css = readFileSync(join(import.meta.dirname, "../room/room.css"), "utf8");
  const style = document.createElement("style");

  style.textContent = css.replace("@layer app {", "").replace(/}\s*$/, "");
  document.head.append(style);

  return style;
}

afterEach(() => {
  document.head.querySelectorAll("style").forEach((style) => {
    style.remove();
  });
  document.documentElement.removeAttribute("data-theme");
});

describe("spoiler styles", () => {
  it("hides every descendant of a covered spoiler, in light and dark, until it is revealed", async () => {
    const user = userEvent.setup();

    addRoomStyles();

    const { container } = render(<BodyHtml html={MENTION_AND_EMOJI} className="message-body" />);
    const covered = container.querySelector<HTMLElement>("[data-spoiler]");
    const name = container.querySelector(".profile-card-name");
    const image = container.querySelector("img");
    const link = container.querySelector("a");

    if (covered === null || name === null || image === null || link === null) {
      throw new Error("expected a spoiler with a mention, an image and a link");
    }

    for (const theme of ["light", "dark"]) {
      document.documentElement.dataset.theme = theme;

      // The mention name's own later rule sets a colour and background. Neither shows.
      for (const node of [name, image, link]) {
        expect(getComputedStyle(node).visibility, `${node.tagName} (${theme})`).toBe("hidden");
      }

      expect(getComputedStyle(covered).visibility).toBe("visible");
    }

    await user.click(covered);

    for (const node of [name, image, link]) {
      expect(getComputedStyle(node).visibility, node.tagName).toBe("visible");
    }
  });
});

describe("revealing", () => {
  it("lets a revealed link open from the keyboard", async () => {
    const user = userEvent.setup();
    const clicks: { readonly detail: number; readonly prevented: boolean }[] = [];

    const record = (event: MouseEvent) => {
      if (event.target instanceof HTMLAnchorElement) {
        clicks.push({ detail: event.detail, prevented: event.defaultPrevented });
        // jsdom can't navigate; the default is what matters.
        event.preventDefault();
      }
    };

    window.addEventListener("click", record);

    try {
      const html = `<p>${spoiler('<a href="https://example.com/ending">ending</a>')}</p>`;

      render(<BodyHtml html={html} className="message-body" />);

      const button = screen.getByRole("button", { name: SPOILER_LABEL });

      button.focus();
      await user.keyboard("{Enter}");

      const link = screen.getByRole("link", { name: "ending" });

      link.focus();
      await user.keyboard("{Enter}");

      expect(clicks).toEqual([{ detail: 0, prevented: false }]);
    } finally {
      window.removeEventListener("click", record);
    }
  });

  it("keeps a still-covered inner spoiler's contents hidden when the outer one is revealed", async () => {
    const user = userEvent.setup();

    const html = `<p>${spoiler(
      `outer ${spoiler('<a href="https://example.com" title="Alice dies">inner</a>')} tail`,
    )}</p>`;

    const { container } = render(<BodyHtml html={html} className="message-body" />);
    const [outer, inner] = container.querySelectorAll<HTMLElement>("[data-spoiler]");
    const link = container.querySelector("a");

    if (outer === undefined || inner === undefined || link === null) {
      throw new Error("expected two spoilers and a link");
    }

    await user.click(outer);

    expect(outer.hasAttribute("data-revealed")).toBe(true);
    expect(inner.hasAttribute("data-revealed")).toBe(false);
    expect(inner.inert).toBe(false);
    expect(inner.tabIndex).toBe(0);
    expect(inner.getAttribute("aria-label")).toBe(SPOILER_LABEL);

    expect(link.getAttribute("title")).toBeNull();
    expect(link.inert).toBe(true);
    expect(link.tabIndex).toBe(-1);
    expect(link.getAttribute("aria-hidden")).toBe("true");
    expect(screen.queryByRole("link")).toBeNull();

    await user.click(inner);

    expect(link.getAttribute("title")).toBe("Alice dies");
    expect(link.inert).toBe(false);
    expect(link.hasAttribute("tabindex")).toBe(false);
    expect(screen.getByRole("link", { name: "inner" })).toBe(link);
  });
});

describe("a link around a spoiler", () => {
  /** `[see ||ending||](https://example.com/alice-dies "Alice dies")` as the renderer writes it. */
  const html =
    '<p><a href="https://example.com/alice-dies" title="Alice dies">see ' +
    `${spoiler("ending")}</a> after</p>`;

  it("has no URL, tooltip, link role or tab stop until its spoiler is revealed", async () => {
    const user = userEvent.setup();
    const { container } = render(<BodyHtml html={html} className="message-body" />);
    const link = container.querySelector("a");

    if (link === null) {
      throw new Error("expected the link");
    }

    expect(link.hasAttribute("href")).toBe(false);
    expect(link.getAttribute("title")).toBeNull();
    expect(screen.queryByRole("link")).toBeNull();

    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: SPOILER_LABEL }));
    await user.tab();
    expect(container.contains(document.activeElement)).toBe(false);

    await user.keyboard("{Shift>}{Tab}{/Shift}{Enter}");

    expect(link.getAttribute("href")).toBe("https://example.com/alice-dies");
    expect(link.getAttribute("title")).toBe("Alice dies");
    expect(screen.getByRole("link", { name: "see ending" })).toBe(link);
  });

  it("reveals instead of opening when the rest of its label is clicked, then opens", async () => {
    const user = userEvent.setup();
    const onRow = vi.fn();
    const clicks: boolean[] = [];

    const record = (event: MouseEvent) => {
      if (event.target instanceof HTMLAnchorElement) {
        clicks.push(event.defaultPrevented);
        event.preventDefault();
      }
    };

    window.addEventListener("click", record);

    try {
      const { container } = render(<BodyHtml html={html} className="message-body" />);
      const link = container.querySelector("a");

      if (link === null) {
        throw new Error("expected the link");
      }

      container.addEventListener("click", onRow);
      await user.click(link);

      expect(container.querySelector("[data-spoiler]")?.hasAttribute("data-revealed")).toBe(true);
      expect(onRow).not.toHaveBeenCalled();
      expect(clicks).toEqual([]);

      link.focus();
      await user.keyboard("{Enter}");

      expect(clicks).toEqual([false]);
    } finally {
      window.removeEventListener("click", record);
    }
  });
});

/** A body that unmounts and mounts again with the same HTML, as an edit that is cancelled. */
function Toggled({ html }: { readonly html: string }) {
  const [shown, setShown] = useState(true);
  const ref = useSpoilerReveal(html);

  return (
    <>
      <button type="button" onClick={() => setShown((value) => !value)}>
        Toggle
      </button>
      {shown ? (
        <div
          ref={ref}
          className="message-body"
          // biome-ignore lint/security/noDangerouslySetInnerHtml: fixed test markup
          dangerouslySetInnerHTML={{ __html: inlineMentions(html) }}
        />
      ) : null}
    </>
  );
}

describe("binding", () => {
  const html = `<p>${spoiler('<a href="https://example.com" title="Alice dies">ending</a>')}</p>`;

  it("covers and reveals the body again when it mounts again with the same HTML", async () => {
    const user = userEvent.setup();

    render(<Toggled html={html} />);

    await user.click(screen.getByRole("button", { name: "Toggle" }));
    await user.click(screen.getByRole("button", { name: "Toggle" }));

    const covered = screen.getByRole("button", { name: SPOILER_LABEL });

    expect(document.querySelector("a")?.getAttribute("title")).toBeNull();
    expect(document.querySelector("a")?.tabIndex).toBe(-1);

    await user.click(covered);

    expect(covered.hasAttribute("data-revealed")).toBe(true);
    expect(document.querySelector("a")?.getAttribute("title")).toBe("Alice dies");
  });

  it("covers new markup when the HTML changes", async () => {
    const user = userEvent.setup();
    const later = `<p>${spoiler("second secret")}</p>`;
    const { rerender } = render(<BodyHtml html={html} className="message-body" />);

    rerender(<BodyHtml html={later} className="message-body" />);

    const covered = screen.getByRole("button", { name: SPOILER_LABEL });

    expect(covered.textContent).toBe("second secret");
    await user.click(covered);
    expect(covered.hasAttribute("data-revealed")).toBe(true);
  });

  it("keeps a work result's spoilers covered and revealable after Edit, then Cancel", async () => {
    const user = userEvent.setup();

    const work = workDetailFixture({
      resultMarkdown: "||[ending](https://example.com)||",
      resultHtml: html,
    });

    render(<WorkResult threadId={1} work={work} updatedAt={null} canEdit announce={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "Edit result" }));
    await user.click(screen.getByRole("button", { name: "Cancel" }));

    const covered = screen.getByRole("button", { name: SPOILER_LABEL });

    expect(document.querySelector("a")?.getAttribute("title")).toBeNull();
    expect(document.querySelector("a")?.tabIndex).toBe(-1);

    await user.click(covered);

    expect(covered.hasAttribute("data-revealed")).toBe(true);
    expect(screen.getByRole("link", { name: "ending" }).getAttribute("title")).toBe("Alice dies");
  });
});
