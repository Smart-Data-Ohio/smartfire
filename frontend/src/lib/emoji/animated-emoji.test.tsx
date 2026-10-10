import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { BodyHtml } from "../../features/messages/body-html.tsx";
import { EmojiImage, stillBodyIcons, stillSrc } from "./emoji-image.tsx";
import EmojiPicker from "./emoji-picker.tsx";
import type { EmojiChoice } from "./recent.ts";

/** Whether the OS asks for reduced motion, as the stubbed `matchMedia` answers. */
let systemReduced = false;

beforeEach(() => {
  systemReduced = false;
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), {
      media,
      matches: media.includes("prefers-reduced-motion") && systemReduced,
      onchange: null,
    }),
  );
});

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.motion;
  vi.unstubAllGlobals();
});

const DANCE: EmojiChoice = {
  content: ":dance:",
  title: "Dance",
  imageUrl: "/icons/dance",
  stillUrl: "/icons/dance?still=1",
};

const DANCE_OFF: EmojiChoice = {
  content: ":danceoff:",
  title: "Dance off",
  imageUrl: "/icons/danceoff",
  stillUrl: "/icons/danceoff?still=1",
};

function srcOf(element: Element | null | undefined): string | null | undefined {
  return element?.querySelector("img")?.getAttribute("src");
}

describe("an animated emoji's first frame", () => {
  it("is the still the server named, else ?still=1 on a workspace icon", () => {
    expect(stillSrc("/icons/dance", "/icons/dance?still=1")).toBe("/icons/dance?still=1");
    expect(stillSrc("/icons/dance")).toBe("/icons/dance?still=1");
    expect(stillSrc("/icons/dance", null)).toBe("/icons/dance?still=1");
    // Brand icons never move.
    expect(stillSrc("/assets/icons/brands/github.svg")).toBe("/assets/icons/brands/github.svg");
  });

  it("replaces only workspace icons in message HTML", () => {
    const html =
      '<p>Go <img class="icon icon--custom" src="/icons/dance" alt=":dance:" title="Dance" draggable="false">' +
      ' <img class="icon icon--brand" src="/assets/icons/brands/github.svg" alt=":github:"></p>';

    expect(stillBodyIcons(html)).toBe(
      html.replace('src="/icons/dance"', 'src="/icons/dance?still=1"'),
    );
    expect(stillBodyIcons("<p>No icons</p>")).toBe("<p>No icons</p>");
  });
});

describe("EmojiImage", () => {
  it("plays an animated emoji", () => {
    const { container } = render(<EmojiImage src={DANCE.imageUrl ?? ""} still={DANCE.stillUrl} />);

    expect(srcOf(container)).toBe("/icons/dance");
  });

  it("shows the first frame when the OS asks for reduced motion", () => {
    systemReduced = true;

    const { container } = render(<EmojiImage src="/icons/dance" still="/icons/dance?still=1" />);

    expect(srcOf(container)).toBe("/icons/dance?still=1");
  });

  it("follows the Smartfire motion setting over the OS, live", async () => {
    systemReduced = true;
    document.documentElement.dataset.motion = "full";

    const { container } = render(<EmojiImage src="/icons/dance" />);

    expect(srcOf(container)).toBe("/icons/dance");

    document.documentElement.dataset.motion = "reduce";

    // Without a known still (a reaction pill), the first frame is asked for with ?still=1.
    await waitFor(() => expect(srcOf(container)).toBe("/icons/dance?still=1"));
  });

  it("rests on a known still, and plays when there is none to rest on", () => {
    const { container, rerender } = render(
      <EmojiImage src="/icons/dance" still="/icons/dance?still=1" resting />,
    );

    expect(srcOf(container)).toBe("/icons/dance?still=1");

    rerender(<EmojiImage src="/icons/dance" resting />);

    expect(srcOf(container)).toBe("/icons/dance");
  });
});

describe("message HTML", () => {
  const html =
    '<p><img class="icon icon--custom" src="/icons/dance" alt=":dance:" title="Dance" draggable="false"></p>';

  it("plays a workspace icon, and shows its first frame under reduced motion", () => {
    const { container, unmount } = render(<BodyHtml html={html} className="message-body" />);

    expect(srcOf(container)).toBe("/icons/dance");
    unmount();

    document.documentElement.dataset.motion = "reduce";

    const reduced = render(<BodyHtml html={html} className="message-body" />);

    expect(srcOf(reduced.container)).toBe("/icons/dance?still=1");
  });
});

describe("the emoji picker", () => {
  beforeEach(() => {
    // Give the virtual grid a viewport, since jsdom has no layout, ResizeObserver or scrollTo.
    Element.prototype.scrollTo = () => undefined;
    vi.spyOn(HTMLElement.prototype, "offsetParent", "get").mockImplementation(function (
      this: HTMLElement,
    ) {
      return this.parentElement;
    });
    vi.stubGlobal(
      "ResizeObserver",
      class implements ResizeObserver {
        private readonly callback: ResizeObserverCallback;

        constructor(callback: ResizeObserverCallback) {
          this.callback = callback;
        }

        observe(target: Element) {
          queueMicrotask(() =>
            this.callback(
              [
                {
                  target,
                  contentRect: new DOMRect(0, 0, 320, 288),
                  borderBoxSize: [],
                  contentBoxSize: [],
                  devicePixelContentBoxSize: [],
                },
              ],
              this,
            ),
          );
        }

        readonly unobserve = vi.fn();
        readonly disconnect = vi.fn();
      },
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  async function openPicker() {
    const user = userEvent.setup();

    render(<EmojiPicker onPick={() => {}} loadCustomIcons={async () => [DANCE, DANCE_OFF]} />);
    await user.type(await screen.findByRole("combobox", { name: "Search emoji" }), "dance");

    const first = await screen.findByRole("option", { name: "Dance" });
    const second = screen.getByRole("option", { name: "Dance off" });

    return { first, second };
  }

  function previewSrc(): string | null | undefined {
    return srcOf(document.querySelector(".emoji-picker-preview"));
  }

  it("plays the active cell and the preview; the other cells rest on their first frame", async () => {
    const { first, second } = await openPicker();

    expect(srcOf(first)).toBe("/icons/dance");
    expect(srcOf(second)).toBe("/icons/danceoff?still=1");
    expect(previewSrc()).toBe("/icons/dance");

    fireEvent.pointerMove(second);

    await waitFor(() => expect(srcOf(second)).toBe("/icons/danceoff"));
    expect(srcOf(first)).toBe("/icons/dance?still=1");
    expect(previewSrc()).toBe("/icons/danceoff");
  });

  it("never plays under reduced motion", async () => {
    document.documentElement.dataset.motion = "reduce";

    const { first, second } = await openPicker();

    expect(srcOf(first)).toBe("/icons/dance?still=1");
    expect(srcOf(second)).toBe("/icons/danceoff?still=1");
    expect(previewSrc()).toBe("/icons/dance?still=1");
  });
});
