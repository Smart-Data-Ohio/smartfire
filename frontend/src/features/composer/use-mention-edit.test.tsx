import { fireEvent, render, screen } from "@testing-library/react";
import { useRef, useState } from "react";
import { describe, expect, it } from "vitest";
import { hydrateMentions, mentionMarkdown } from "./mentions.ts";
import { useMentionEdit } from "./use-mention-edit.ts";

function Editor({ source }: { readonly source: string }) {
  const [draft, setDraft] = useState(() =>
    hydrateMentions(source, (id) => (id === 14 ? "Jon" : "Twin")),
  );

  const ref = useRef<HTMLTextAreaElement | null>(null);
  const edit = useMentionEdit(ref, draft);

  return (
    <>
      <textarea
        ref={ref}
        aria-label="Draft"
        value={draft.text}
        onChange={(event) => setDraft(edit(draft, event.target.value, event.target.selectionStart))}
      />
      <output>{mentionMarkdown(draft)}</output>
    </>
  );
}

function input(text: string, inputType: string, start: number, end = start) {
  const box = screen.getByRole<HTMLTextAreaElement>("textbox", { name: "Draft" });
  box.setSelectionRange(start, end);
  fireEvent(box, new InputEvent("beforeinput", { bubbles: true, inputType }));
  fireEvent.change(box, { target: { value: text, selectionStart: start, selectionEnd: start } });
}

function sent() {
  return screen.getByRole("status").textContent;
}

describe("mention input history", () => {
  it("restores an edited mention on undo and degrades it again on redo", () => {
    render(<Editor source="<@14>" />);
    input("@Joxn", "insertText", 3);
    expect(sent()).toBe("@Joxn");
    input("@Jon", "historyUndo", 4);
    expect(sent()).toBe("<@14>");
    input("@Joxn", "historyRedo", 3);
    expect(sent()).toBe("@Joxn");
  });

  it("restores a mention after typing beyond the snapshot window and undoing twice", () => {
    render(<Editor source="<@14>" />);
    input("@Joxn", "insertText", 3);

    let text = "@Joxn ";

    for (let count = 0; count < 120; count += 1) {
      const start = text.length;
      text += "x";
      input(text, "insertText", start);
    }

    expect(sent()).toBe(`@Joxn ${"x".repeat(120)}`);
    input("@Joxn", "historyUndo", text.length);
    expect(sent()).toBe("@Joxn");
    input("@Jon", "historyUndo", 5);
    expect(sent()).toBe("<@14>");
  });

  it("undoes to the latest mention snapshot when every retained snapshot has mentions", () => {
    render(<Editor source="<@14>" />);
    let text = "@Jon ";

    for (let count = 0; count < 120; count += 1) {
      const start = text.length;
      text += "x";
      input(text, "insertText", start);
    }

    input(`${text}!`, "insertFromPaste", 0, text.length);
    expect(sent()).toBe(`${text}!`);
    input(text, "historyUndo", text.length + 1);
    expect(sent()).toBe(`<@14> ${"x".repeat(120)}`);
  });

  it("restores twin identities after deleting the first, undoing and redoing", () => {
    render(<Editor source="<@123> <@456>" />);
    input("@Twin", "deleteContentBackward", 0, 6);
    expect(sent()).toBe("<@456>");
    input("@Twin @Twin", "historyUndo", 0);
    expect(sent()).toBe("<@123> <@456>");
    input("@Twin", "historyRedo", 0);
    expect(sent()).toBe("<@456>");
  });

  it("discards a redo branch so identical text restores the nearest twin identity", () => {
    render(<Editor source="<@123> <@456>" />);
    input("@Twin", "deleteContentBackward", 0, 6);
    expect(sent()).toBe("<@456>");
    input("@Twin @Twin", "historyUndo", 0);
    input("@Twin", "deleteContentBackward", 5, 11);
    expect(sent()).toBe("<@123>");
    input("@Twin @Twin", "historyUndo", 5);
    expect(sent()).toBe("<@123> <@456>");
    input("@Twin", "historyRedo", 5);
    expect(sent()).toBe("<@123>");
  });

  it("restores mention identity when a composition is cancelled", () => {
    render(<Editor source="<@14>" />);
    const box = screen.getByRole("textbox", { name: "Draft" });
    fireEvent.compositionStart(box);
    input("@Joxn", "insertCompositionText", 3);
    expect(sent()).toBe("@Joxn");
    input("@Jon", "insertCompositionText", 3, 4);
    expect(sent()).toBe("<@14>");
    fireEvent.compositionEnd(box);
  });

  it("keeps a deleted mention literal when its name is retyped through composition", () => {
    render(<Editor source="<@14>" />);
    input("", "deleteContentBackward", 0, 4);
    expect(sent()).toBe("");

    const box = screen.getByRole("textbox", { name: "Draft" });
    fireEvent.compositionStart(box);
    input("@Jo", "insertCompositionText", 0);
    input("@Jon", "insertCompositionText", 0, 3);
    expect(sent()).toBe("@Jon");
    fireEvent.compositionEnd(box);
  });

  it("uses the selection when pasted text replaces a mention", () => {
    render(<Editor source="<@123> <@456>" />);
    input("hello @Twin", "insertFromPaste", 0, 5);
    expect(sent()).toBe("hello <@456>");
  });

  it("falls back to a diff when history has no matching snapshot", () => {
    render(<Editor source="<@14>" />);
    input("Hi @Jon", "historyUndo", 4);
    expect(sent()).toBe("Hi <@14>");
  });
});
