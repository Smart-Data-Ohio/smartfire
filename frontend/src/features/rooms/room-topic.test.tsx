import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { RoomTopic } from "./room-topic.tsx";

describe("plain channel topics", () => {
  it.each([
    ["See [https://example.com/docs].", "https://example.com/docs", "]."],
    ["See (https://example.com/docs.)", "https://example.com/docs", ".)"],
    ["See ([https://example.com/docs!?]).", "https://example.com/docs", "!?])."],
    ["See {https://example.com/docs,}", "https://example.com/docs", ",}"],
    ["See (https://example.com/wiki/(topic).)", "https://example.com/wiki/(topic)", ".)"],
    ["See [https://example.com/docs[topic]].", "https://example.com/docs[topic]", "]."],
    ["See https://example.com/a)(topic).", "https://example.com/a)(topic)", "."],
  ])("keeps surrounding punctuation outside the link in %s", (topic, href, suffix) => {
    const { container } = render(<RoomTopic topic={topic} />);
    const link = screen.getByRole("link", { name: href });

    expect(link.getAttribute("href")).toBe(href);
    expect(link.nextSibling?.textContent).toBe(suffix);
    expect(container.textContent).toBe(topic);
  });

  it("autolinks web URLs while preserving punctuation, newlines and literal markup", () => {
    const topic =
      "<script>alert(1)</script>\nwww.example.com, https://example.com/wiki/(topic). javascript:alert(1)";

    const { container } = render(<RoomTopic topic={topic} />);
    expect(container.textContent).toBe(topic);
    expect(container.querySelector("script")).toBeNull();
    expect(screen.getByRole("link", { name: "www.example.com" }).getAttribute("href")).toBe(
      "https://www.example.com",
    );
    expect(
      screen.getByRole("link", { name: "https://example.com/wiki/(topic)" }).getAttribute("href"),
    ).toBe("https://example.com/wiki/(topic)");
    expect(screen.getAllByRole("link")).toHaveLength(2);
  });
});
