import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Field } from "./event-form-dialog.tsx";

/** The text a control's `aria-describedby` points at. */
function description(control: HTMLElement): string {
  return (control.getAttribute("aria-describedby") ?? "")
    .split(" ")
    .map((id) => document.getElementById(id)?.textContent ?? "")
    .join(" ")
    .trim();
}

describe("the event form's own fields", () => {
  it("tie a select's and a text area's error to them and mark them invalid", () => {
    render(
      <>
        <Field id="rule" label="Repeats" error="Repeats can't be removed from a repeating event.">
          {(control) => (
            <select {...control}>
              <option>Weekly</option>
            </select>
          )}
        </Field>
        <Field id="notes" label="Description (optional)" error="Description is too long.">
          {(control) => <textarea {...control} />}
        </Field>
      </>,
    );

    const repeats = screen.getByLabelText("Repeats");
    const notes = screen.getByLabelText("Description (optional)");

    expect(repeats.getAttribute("aria-invalid")).toBe("true");
    expect(description(repeats)).toBe("Repeats can't be removed from a repeating event.");
    expect(notes.getAttribute("aria-invalid")).toBe("true");
    expect(description(notes)).toBe("Description is too long.");
  });

  it("describe a field by its hint while it has no error", () => {
    render(
      <Field id="venue" label="Where" hint="A voice channel or stage.">
        {(control) => <select {...control} />}
      </Field>,
    );

    const venue = screen.getByLabelText("Where");

    expect(venue.hasAttribute("aria-invalid")).toBe(false);
    expect(description(venue)).toBe("A voice channel or stage.");
  });
});
