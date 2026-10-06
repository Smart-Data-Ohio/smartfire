import { describe, expect, it } from "vitest";
import type { ForwardDestination } from "../../gen/ForwardDestination.ts";
import { fileIcon, fileKind, fitWithin, formatBytes } from "./format.ts";
import { filterDestinations, targetKey } from "./forward-targets.ts";
import { reactionLabel, reactorSummary } from "./reaction-summary.ts";

describe("reaction tooltips", () => {
  const names = new Map([
    [2, "Maya Okafor"],
    [3, "Theo Brandt"],
    [4, "Priya Raman"],
  ]);

  const nameOf = (id: number) => names.get(id);

  it("names the viewer first as You", () => {
    expect(reactorSummary([2, 1], 1, nameOf, "Fire")).toBe("You and Maya Okafor reacted with Fire");
    expect(reactorSummary([1], 1, nameOf, "Fire")).toBe("You reacted with Fire");
  });

  it("lists names with commas and counts the people not loaded yet", () => {
    expect(reactorSummary([2, 3, 4], 1, nameOf, "Tada")).toBe(
      "Maya Okafor, Theo Brandt and Priya Raman reacted with Tada",
    );
    expect(reactorSummary([2, 99], 1, nameOf, "Tada")).toBe(
      "Maya Okafor and 1 other reacted with Tada",
    );
    expect(reactorSummary([98, 99], null, nameOf, "Tada")).toBe("2 people reacted with Tada");
  });

  it("caps the named list at six", () => {
    const many = Array.from({ length: 9 }, (_, index) => index + 10);
    const summary = reactorSummary(many, null, (id) => `P${id}`, "Eyes");

    expect(summary).toBe("P10, P11, P12, P13, P14, P15 and 3 others reacted with Eyes");
  });

  it("labels a pill for screen readers", () => {
    expect(reactionLabel("Thumbs up", 3, true)).toBe("Thumbs up: 3 reactions, including yours");
    expect(reactionLabel("Thumbs up", 1, false)).toBe("Thumbs up: 1 reaction");
  });
});

describe("attachment formatting", () => {
  it("formats sizes", () => {
    expect(formatBytes(1)).toBe("1 byte");
    expect(formatBytes(512)).toBe("512 bytes");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(34 * 1024 * 1024)).toBe("34 MB");
  });

  it("picks an icon and a kind label", () => {
    expect(fileIcon("application/pdf")).toBe("file-text");
    expect(fileIcon("application/zip")).toBe("file-archive");
    expect(fileIcon("audio/mpeg")).toBe("music");
    expect(fileIcon("application/octet-stream")).toBe("file");
    expect(fileKind("Q3-board-update.pdf", "application/pdf")).toBe("PDF");
    expect(fileKind("notes", "text/plain")).toBe("PLAIN");
    expect(fileKind("blob", "application/octet-stream")).toBe("File");
  });

  it("fits media inside the box without enlarging it", () => {
    expect(fitWithin(1200, 675, { width: 400, height: 300 })).toEqual({ width: 400, height: 225 });
    expect(fitWithin(100, 50, { width: 400, height: 300 })).toEqual({ width: 100, height: 50 });
    expect(fitWithin(null, 50, { width: 400, height: 300 })).toBeNull();
  });
});

describe("forward destinations", () => {
  const destinations: ForwardDestination[] = [
    {
      roomId: 1,
      name: "general",
      direct: false,
      threads: [
        { id: 7, name: "Launch checklist", status: "active" },
        { id: 8, name: "Pricing", status: "active" },
      ],
    },
    {
      roomId: 2,
      name: "design",
      direct: false,
      threads: [{ id: 9, name: "Launch visuals", status: "active" }],
    },
    { roomId: 3, name: "Maya Okafor", direct: true, threads: [] },
  ];

  it("keys rooms and threads apart", () => {
    expect(targetKey({ roomId: 1, threadId: null })).toBe("1");
    expect(targetKey({ roomId: 1, threadId: 7 })).toBe("1:7");
  });

  it("keeps a matching room whole and narrows others to matching threads", () => {
    expect(filterDestinations(destinations, "")).toHaveLength(3);
    expect(filterDestinations(destinations, "GEN")[0]?.threads).toHaveLength(2);

    const launch = filterDestinations(destinations, "launch");

    expect(launch.map((entry) => entry.roomId)).toEqual([1, 2]);
    expect(launch[0]?.threads.map((thread) => thread.id)).toEqual([7]);
    expect(filterDestinations(destinations, "nothing")).toEqual([]);
  });
});
