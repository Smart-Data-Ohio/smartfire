import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PagedList, type PagedState } from "./paged-list.tsx";

function state(overrides: Partial<PagedState>): PagedState {
  return {
    status: "ready",
    loadingMore: false,
    hasMore: false,
    error: null,
    loadMore: vi.fn(),
    reload: vi.fn(),
    ...overrides,
  };
}

function renderList(paged: PagedState) {
  render(
    <PagedList
      state={paged}
      label="Saved messages"
      errorText="Your saved messages couldn't be loaded."
      isEmpty
      empty={<p>Nothing saved for later</p>}
    >
      {[]}
    </PagedList>,
  );
}

describe("a paged list with no rows loaded", () => {
  it("shows the empty state once the server has nothing more", () => {
    const paged = state({ hasMore: false });

    renderList(paged);

    expect(screen.getByText("Nothing saved for later")).toBeTruthy();
    expect(paged.loadMore).not.toHaveBeenCalled();
  });

  it("keeps loading, not caught up, when rows left locally and more wait", () => {
    const paged = state({ hasMore: true });

    renderList(paged);

    expect(screen.queryByText("Nothing saved for later")).toBeNull();
    expect(document.querySelector(".t-skel")?.getAttribute("aria-busy")).toBe("true");
    expect(paged.loadMore).toHaveBeenCalledTimes(1);
  });

  it("offers Try again when loading the rest failed", () => {
    const paged = state({ hasMore: true, error: "Couldn't load more" });

    renderList(paged);

    expect(screen.queryByText("Nothing saved for later")).toBeNull();
    expect(screen.getByRole("alert").textContent).toContain("Couldn't load more");
    expect(screen.getByRole("button", { name: "Try again" })).toBeTruthy();
    expect(paged.loadMore).not.toHaveBeenCalled();
  });
});
