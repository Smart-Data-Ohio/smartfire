import { render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cardMutations } from "../../store/card-mutations.ts";
import { PREVIEW_TTL_MS, quoteKey } from "../../store/cards.ts";
import { mutations } from "../../store/store.ts";
import { usePreview } from "./use-preview.ts";

const KEY = quoteKey(5, 3);

/** Whether a component is in its render phase, so the clock can tell a read there from one later. */
let rendering = false;

function Probe({ load }: { readonly load: () => Promise<void> }) {
  rendering = true;

  const preview = usePreview("quotes", KEY, load);

  rendering = false;

  return <span>{preview?.status ?? "none"}</span>;
}

afterEach(() => {
  vi.restoreAllMocks();
  mutations.reset();
});

describe("usePreview", () => {
  it("fetches a missing or stale preview without reading the clock while rendering", () => {
    const readDuringRender: number[] = [];
    const now = Date.now();

    vi.spyOn(Date, "now").mockImplementation(() => {
      if (rendering) {
        readDuringRender.push(now);
      }

      return now;
    });

    const load = vi.fn(() => Promise.resolve());

    cardMutations.previewLoaded("quotes", KEY, 3, { state: "hidden" }, now - PREVIEW_TTL_MS - 1);

    const view = render(<Probe load={load} />);

    expect(load).toHaveBeenCalledTimes(1);
    expect(readDuringRender).toEqual([]);

    // A fresh one stays as it is.
    view.unmount();
    load.mockClear();
    cardMutations.previewLoaded("quotes", KEY, 3, { state: "hidden" }, now);
    render(<Probe load={load} />);
    expect(load).not.toHaveBeenCalled();
  });
});
