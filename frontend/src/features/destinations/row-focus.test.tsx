import { render, screen, waitFor } from "@testing-library/react";
import { useRef } from "react";
import { describe, expect, it } from "vitest";
import { landingFor, placeOf, useKeepRowFocus } from "./row-focus.ts";

interface HarnessProps {
  readonly rows: readonly number[];
  readonly leaving?: number;
  /** A menu (outside the list) holding focus, as a row's menu or dialog would. */
  readonly menu?: boolean;
  readonly disabled?: number;
}

function Harness({ rows, leaving, menu = false, disabled }: HarnessProps) {
  const rootRef = useRef<HTMLDivElement | null>(null);

  useKeepRowFocus(rootRef);

  return (
    <>
      <div ref={rootRef}>
        {rows.length === 0 ? (
          <p>All caught up</p>
        ) : (
          <ul aria-label="Rows" data-list-root="">
            {rows.map((id) => (
              <li
                key={id}
                className="list-row"
                data-motion={id === leaving ? "leave" : undefined}
                inert={id === leaving}
              >
                <button type="button" className="list-row-open" disabled={id === disabled}>
                  Row {id}
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
      <button type="button">Elsewhere</button>
      {menu ? (
        <div role="menu">
          <button type="button">Mark as handled</button>
        </div>
      ) : null}
    </>
  );
}

function row(id: number): HTMLElement {
  return screen.getByRole("button", { name: `Row ${id}` });
}

describe("keeping focus as rows leave", () => {
  it("moves to the next row when the focused row leaves", () => {
    const { rerender } = render(<Harness rows={[1, 2, 3]} />);

    row(2).focus();
    rerender(<Harness rows={[1, 2, 3]} leaving={2} />);

    expect(document.activeElement).toBe(row(3));
  });

  it("falls back to the previous row at the end of the list", () => {
    const { rerender } = render(<Harness rows={[1, 2, 3]} />);

    row(3).focus();
    rerender(<Harness rows={[1, 2, 3]} leaving={3} />);

    expect(document.activeElement).toBe(row(2));
  });

  it("skips a row whose open button is disabled", () => {
    const { rerender } = render(<Harness rows={[1, 2, 3]} disabled={3} />);

    row(2).focus();
    rerender(<Harness rows={[1, 2, 3]} leaving={2} disabled={3} />);

    expect(document.activeElement).toBe(row(1));
  });

  it("moves on when the focused row is removed outright", () => {
    const { rerender } = render(<Harness rows={[1, 2, 3]} />);

    row(2).focus();
    rerender(<Harness rows={[1, 3]} />);

    expect(document.activeElement).toBe(row(3));
  });

  it("holds focus on the list when no row is left, then on its container", () => {
    const { rerender, container } = render(<Harness rows={[1]} />);

    row(1).focus();
    rerender(<Harness rows={[1]} leaving={1} />);

    const list = screen.getByRole("list", { name: "Rows" });

    expect(document.activeElement).toBe(list);
    expect(list.tabIndex).toBe(-1);

    rerender(<Harness rows={[]} />);

    expect(document.activeElement).toBe(container.firstElementChild);
    expect(screen.getByText("All caught up")).toBeTruthy();
  });

  it("waits for a menu opened from the row to close, then lands on the next row", async () => {
    const { rerender } = render(<Harness rows={[1, 2, 3]} />);

    row(2).focus();
    rerender(<Harness rows={[1, 2, 3]} menu />);
    screen.getByRole("button", { name: "Mark as handled" }).focus();
    rerender(<Harness rows={[1, 2, 3]} leaving={2} menu />);

    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Mark as handled" }));

    rerender(<Harness rows={[1, 3]} />);

    await waitFor(() => expect(document.activeElement).toBe(row(3)));
  });

  it("lands once the menu goes even if the list doesn't render again", async () => {
    const { rerender } = render(<Harness rows={[1, 2, 3]} />);
    const menu = document.createElement("div");
    const item = document.createElement("button");

    menu.setAttribute("role", "menu");
    menu.append(item);
    document.body.append(menu);
    row(2).focus();
    item.focus();
    rerender(<Harness rows={[1, 2, 3]} leaving={2} />);

    expect(document.activeElement).toBe(item);

    menu.remove();

    await waitFor(() => expect(document.activeElement).toBe(row(3)));
  });

  it("leaves focus alone once it has moved elsewhere", () => {
    const { rerender } = render(<Harness rows={[1, 2, 3]} />);
    const elsewhere = screen.getByRole("button", { name: "Elsewhere" });

    row(2).focus();
    elsewhere.focus();
    rerender(<Harness rows={[1, 2, 3]} leaving={2} />);

    expect(document.activeElement).toBe(elsewhere);
  });
});

describe("where focus lands", () => {
  it("picks the next live row, then the previous, then the list", () => {
    render(<Harness rows={[1, 2, 3]} leaving={3} />);

    const root = screen.getByRole("list", { name: "Rows" });
    const rows = [...root.querySelectorAll(".list-row")];
    const [first, second, third] = rows;

    if (first === undefined || second === undefined || third === undefined) {
      throw new Error("three rows expected");
    }

    expect(landingFor(root, placeOf(root, second))).toBe(row(1));
    expect(landingFor(root, placeOf(root, first))).toBe(row(2));
    expect(landingFor(root, { row: first, before: [], after: [third] })).toBe(root);
  });
});
