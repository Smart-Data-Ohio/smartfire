import { afterEach, describe, expect, it, vi } from "vitest";
import { switchToClassic } from "./user-menu.tsx";

afterEach(() => {
  for (const node of document.querySelectorAll("meta[name^='csrf-'], form")) {
    node.remove();
  }

  vi.restoreAllMocks();
});

function meta(name: string, content: string): void {
  const element = document.createElement("meta");

  element.name = name;
  element.content = content;
  document.head.append(element);
}

describe("Switch to classic", () => {
  it("posts the choice with the session's token and where the person is", () => {
    meta("csrf-param", "authenticity_token");
    meta("csrf-token", "tok");
    const submit = vi.spyOn(HTMLFormElement.prototype, "submit").mockImplementation(() => {});

    switchToClassic({ pathname: "/app/r/5/t/9", search: "?m=3" });

    const form = document.querySelector("form");

    expect(submit).toHaveBeenCalledOnce();
    expect(form?.getAttribute("method")).toBe("post");
    expect(form?.getAttribute("action")).toBe("/app/ui_preference");
    expect(Object.fromEntries(new FormData(form ?? undefined))).toEqual({
      ui: "classic",
      return_to: "/app/r/5/t/9?m=3",
      authenticity_token: "tok",
    });
  });
});
