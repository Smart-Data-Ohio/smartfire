import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { SlackIssue } from "../../gen/SlackIssue.ts";
import { Issues } from "./slack-run.tsx";

const issue = (message: string): SlackIssue => ({ level: "warning", slackRef: null, message });

describe("a run's issues", () => {
  it("move focus to the first older issue once the last page is in", async () => {
    let finish: () => void = () => undefined;

    const more = () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      });

    const first = [issue("one"), issue("two")];
    const { rerender } = render(<Issues count={3} issues={first} more onMore={more} />);

    fireEvent.click(screen.getByRole("button", { name: "Older issues" }));

    await act(async () => {
      rerender(<Issues count={3} issues={[...first, issue("three")]} more={false} onMore={more} />);
      finish();
    });

    expect(screen.queryByRole("button", { name: "Older issues" })).toBeNull();
    expect(document.activeElement).toBe(screen.getByText(/three/).closest("li"));
  });

  it("leave focus on the button while older pages remain", async () => {
    let finish: () => void = () => undefined;

    const more = () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      });

    const first = [issue("one")];
    const { rerender } = render(<Issues count={3} issues={first} more onMore={more} />);
    const button = screen.getByRole("button", { name: "Older issues" });

    button.focus();
    fireEvent.click(button);

    await act(async () => {
      rerender(<Issues count={3} issues={[...first, issue("two")]} more onMore={more} />);
      finish();
    });

    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Older issues" }));
  });
});
