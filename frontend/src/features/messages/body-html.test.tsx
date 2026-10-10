import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { inlineMentions } from "../../lib/body-html.ts";
import { BodyHtml } from "./body-html.tsx";

/** A body with a mention as the server renders it: a block `<div>` with an avatar link and name button. */
const HTML =
  '<p>Ship it, <action-text-attachment content-type="application/vnd.campfire.mention" sgid="s1">' +
  '<div class="mention mention--user-7" sgid="s1" data-user-id="7">\n' +
  '  <a title="Maya Okafor" class="btn avatar" href="/users/7">M</a>\n' +
  '  <button name="button" type="button" class="profile-card-name">Maya Okafor</button>\n' +
  "</div>\n</action-text-attachment> &amp; friends</p>";

/** The markup a body rendered before it went through `BodyHtml`, built the way it was. */
function inlined(className: string, stale?: true): string {
  const { container } = render(
    <div
      className={className}
      data-stale={stale}
      // biome-ignore lint/security/noDangerouslySetInnerHtml: the same server HTML, for comparison
      dangerouslySetInnerHTML={{ __html: inlineMentions(HTML) }}
    />,
  );

  return container.innerHTML;
}

describe("BodyHtml", () => {
  it("renders a pinned message's body as the pins pane did", () => {
    const expected = inlined("pin-card-body message-body");

    const { container } = render(<BodyHtml html={HTML} className="pin-card-body message-body" />);

    expect(container.innerHTML).toBe(expected);

    expect(container.querySelector("p .mention .profile-card-name")?.textContent).toBe(
      "Maya Okafor",
    );

    expect(container.querySelector("div.mention, a, button")).toBeNull();
  });

  it("renders the composer preview, stale or not, as the preview panel did", () => {
    for (const stale of [true, undefined] as const) {
      const expected = inlined("message-body composer-preview-body", stale);

      const { container } = render(
        <BodyHtml html={HTML} className="message-body composer-preview-body" data-stale={stale} />,
      );

      expect(container.innerHTML).toBe(expected);
    }
  });

  it("colours a fenced block after mount and keeps it through an unrelated re-render", async () => {
    const html = '<pre><code class="language-ts">const value: string = "hello";</code></pre>';
    const { container, rerender } = render(<BodyHtml html={html} className="message-body" />);

    await vi.waitFor(() => expect(container.querySelector(".code-token--keyword")).not.toBeNull());

    const coloured = container.querySelector("code");

    rerender(<BodyHtml html={html} className="message-body" data-stale />);

    expect(container.querySelector("code")).toBe(coloured);
    expect(container.querySelector(".code-token--string")?.textContent).toBe('"hello"');
    expect(container.querySelectorAll("button.code-copy")).toHaveLength(1);
  });

  it("redoes the blocks when the body changes (an edit)", async () => {
    const before = '<pre><code class="language-rust">fn a() {}</code></pre>';
    const after = '<pre><code class="language-rust">fn b() {}</code></pre>';
    const { container, rerender } = render(<BodyHtml html={before} className="message-body" />);

    await vi.waitFor(() => expect(container.querySelector(".code-token")).not.toBeNull());
    rerender(<BodyHtml html={after} className="message-body" />);
    await vi.waitFor(() => expect(container.querySelector(".code-token")).not.toBeNull());

    expect(container.querySelector("code")?.textContent).toBe("fn b() {}");
    expect(container.querySelectorAll("button.code-copy")).toHaveLength(1);
  });

  it("reveals one spoiler on click and leaves the others covered", async () => {
    const user = userEvent.setup();
    const onAction = vi.fn();

    const html =
      '<p><span class="spoiler" data-spoiler="">secret one</span> ' +
      '<span class="spoiler" data-spoiler="">secret two</span> ' +
      '<span class="spoiler" data-spoiler="">secret three</span></p>';

    const { container } = render(<BodyHtml html={html} className="message-body" />);

    container.addEventListener("click", onAction);
    container.addEventListener("keydown", onAction);

    const [first, second, third] = spoilerElements(container);

    expect(first.getAttribute("aria-label")).toBe("Spoiler, activate to reveal");
    expect(first.getAttribute("role")).toBe("button");
    expect(first.textContent).toBe("secret one");

    await user.click(first);

    expect(first.hasAttribute("data-revealed")).toBe(true);
    expect(first.hasAttribute("aria-label")).toBe(false);
    expect(first.textContent).toBe("secret one");
    expect(second.hasAttribute("data-revealed")).toBe(false);
    expect(onAction).not.toHaveBeenCalled();

    second.focus();
    await user.keyboard("{Enter}");

    expect(second.hasAttribute("data-revealed")).toBe(true);
    expect(second.textContent).toBe("secret two");
    expect(third.hasAttribute("data-revealed")).toBe(false);
    expect(third.getAttribute("aria-label")).toBe("Spoiler, activate to reveal");

    third.focus();
    await user.keyboard(" ");

    expect(third.hasAttribute("data-revealed")).toBe(true);
    expect(onAction).not.toHaveBeenCalled();
  });

  it("keeps a concealed link untitled, out of tab order, and unnamed until it is revealed", async () => {
    const user = userEvent.setup();

    const html =
      '<p>before <span class="spoiler" data-spoiler="">' +
      '<a href="https://example.com" title="Alice dies">ending</a></span> after</p>';

    render(<BodyHtml html={html} className="message-body" />);

    const spoiler = screen.getByRole("button", { name: "Spoiler, activate to reveal" });
    const link = document.querySelector("a");

    expect(link).not.toBeNull();

    if (link === null) {
      return;
    }

    expect(link.getAttribute("title")).toBeNull();
    expect(link.tabIndex).toBe(-1);
    expect(link.inert).toBe(true);
    expect(screen.queryByRole("link", { name: "ending" })).toBeNull();
    expect(screen.queryByRole("link", { name: "Alice dies" })).toBeNull();

    await user.click(spoiler);

    expect(spoiler.hasAttribute("data-revealed")).toBe(true);
    expect(link.getAttribute("title")).toBe("Alice dies");
    expect(link.inert).toBe(false);
    expect(link.hasAttribute("tabindex")).toBe(false);
    expect(screen.getByRole("link", { name: "ending" })).toBe(link);
  });

  it("keeps a mention inside the spoiler and drops its link and button", () => {
    const html =
      '<p><span class="spoiler" data-spoiler="">' +
      '<span class="mention mention--user-1">' +
      '<a class="btn avatar" href="/users/1" title="David – Founder">D</a>' +
      '<button class="profile-card-name" type="button">David</button>' +
      "</span> is the killer</span></p>";

    const { container } = render(<BodyHtml html={html} className="message-body" />);
    const spoiler = container.querySelector("[data-spoiler]");

    expect(spoiler?.textContent).toContain("David");
    expect(spoiler?.textContent).toContain("is the killer");
    expect(spoiler?.querySelector(".mention .profile-card-name")?.textContent).toBe("David");
    expect(container.querySelector("a, button")).toBeNull();
  });
});

function spoilerElements(container: HTMLElement): [HTMLElement, HTMLElement, HTMLElement] {
  const spoilers = [...container.querySelectorAll<HTMLElement>("[data-spoiler]")];
  const [first, second, third] = spoilers;

  if (first === undefined || second === undefined || third === undefined) {
    throw new Error(`expected 3 spoilers, found ${spoilers.length}`);
  }

  return [first, second, third];
}
