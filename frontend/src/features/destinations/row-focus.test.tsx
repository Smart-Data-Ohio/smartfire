import { render, screen, waitFor } from "@testing-library/react";
import { useRef } from "react";
import { describe, expect, it } from "vitest";
import { edgeRowOpen, landingFor, placeOf, type RowParts, useKeepRowFocus } from "./row-focus.ts";

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

/** Rows that name themselves and their controls, as the admin lists do. */
const NAMED: RowParts = {
  list: ".named-root",
  row: "[data-row]",
  open: "[data-row-control]",
  leaving: "[data-motion='leave']",
  id: "data-row",
  control: "data-row-control",
};

interface NamedRow {
  readonly id: number;
  /** Which of the two lists it's in: moving it remounts it. */
  readonly list: "admins" | "members";
  /** Whether it still has its Remove control (a revoked row loses it). */
  readonly removable?: boolean;
}

function NamedHarness({
  rows,
  busy = false,
}: {
  readonly rows: readonly NamedRow[];
  readonly busy?: boolean;
}) {
  const rootRef = useRef<HTMLDivElement | null>(null);

  useKeepRowFocus(rootRef, NAMED);

  const list = (name: NamedRow["list"]) => (
    <ul aria-label={name}>
      {rows
        .filter((each) => each.list === name)
        .map((each) => (
          <li
            key={`${name}-${each.id}`}
            data-row={each.id}
            tabIndex={-1}
            aria-label={`Person ${each.id}`}
          >
            <button type="button" data-row-control="role" disabled={busy}>
              Role {each.id}
            </button>
            {each.removable === false ? null : (
              <button type="button" data-row-control="remove" disabled={busy}>
                Remove {each.id}
              </button>
            )}
          </li>
        ))}
    </ul>
  );

  return (
    <div ref={rootRef} className="named-root" tabIndex={-1}>
      {list("admins")}
      {list("members")}
    </div>
  );
}

const control = (name: string) => screen.getByRole("button", { name });

describe("keeping focus in rows that name themselves", () => {
  it("lands on the same control in the next row when a row goes", () => {
    const members: NamedRow[] = [1, 2, 3].map((id) => ({ id, list: "members" }));
    const { rerender } = render(<NamedHarness rows={members} />);

    control("Remove 2").focus();
    rerender(<NamedHarness rows={members.filter((each) => each.id !== 2)} />);

    expect(document.activeElement).toBe(control("Remove 3"));
  });

  it("follows a row that moves to the other list, once its control is free again", () => {
    const before: NamedRow[] = [
      { id: 1, list: "admins" },
      { id: 2, list: "members" },
      { id: 3, list: "members" },
    ];

    const after: NamedRow[] = [
      { id: 1, list: "admins" },
      { id: 2, list: "admins" },
      { id: 3, list: "members" },
    ];

    const { rerender } = render(<NamedHarness rows={before} />);

    control("Role 2").focus();
    // The save disables the controls, and the browser drops focus from the disabled switch.
    rerender(<NamedHarness rows={before} busy />);
    control("Role 2").blur();
    rerender(<NamedHarness rows={after} busy />);
    expect(document.activeElement).toBe(document.body);

    rerender(<NamedHarness rows={after} />);
    expect(document.activeElement).toBe(control("Role 2"));
  });

  it("stays on the row when the control it was in goes away", () => {
    const second: NamedRow = { id: 2, list: "members" };
    const { rerender } = render(<NamedHarness rows={[{ id: 1, list: "members" }, second]} />);

    control("Remove 1").focus();
    rerender(<NamedHarness rows={[{ id: 1, list: "members", removable: false }, second]} />);

    // The row's first control is its open button.
    expect(document.activeElement).toBe(control("Role 1"));
  });

  it("holds focus on the container when the last row goes", () => {
    const { rerender, container } = render(<NamedHarness rows={[{ id: 1, list: "members" }]} />);

    control("Remove 1").focus();
    rerender(<NamedHarness rows={[]} />);

    expect(document.activeElement).toBe(container.firstElementChild);
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

describe("the edge rows", () => {
  it("are the first and last rows that can take focus", () => {
    render(<Harness rows={[1, 2, 3, 4]} leaving={4} disabled={1} />);

    const root = screen.getByRole("list", { name: "Rows" });

    expect(edgeRowOpen(root, "first")).toBe(row(2));
    expect(edgeRowOpen(root, "last")).toBe(row(3));
  });
});
