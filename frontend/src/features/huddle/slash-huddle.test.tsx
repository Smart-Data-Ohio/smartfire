import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { SEED_IDS } from "../../../mock/server.ts";
import { sidebarFixture, sidebarRowFixture } from "../../api/testing.ts";
import type { Me } from "../../gen/Me.ts";
import { mutations } from "../../store/store.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { toastSnapshot } from "../../ui/toast-store.ts";
import { Composer } from "../composer/composer.tsx";
import { callController } from "./call-controller.ts";
import { startHuddleFromCommand } from "./slash-huddle.ts";

const ROOM = SEED_IDS.rooms.general;

/** Records the joins `/huddle` asks for. */
function recorder() {
  const joins: [number, string, boolean | null][] = [];

  const join = (roomId: number, roomName: string, hint: boolean | null) => {
    joins.push([roomId, roomName, hint]);

    return Promise.resolve();
  };

  return { joins, join };
}

describe("/huddle's start_huddle", () => {
  beforeEach(() => mutations.reset());

  it("joins the room's call, as the call button does", () => {
    const { joins, join } = recorder();

    mutations.loadSidebar(sidebarFixture([sidebarRowFixture(12, "general")]));

    expect(startHuddleFromCommand(12, "general", join)).toEqual({ kind: "joining" });
    expect(joins).toEqual([[12, "general", null]]);
  });

  it("joins a direct message's call", () => {
    const { joins, join } = recorder();

    mutations.loadSidebar(sidebarFixture([sidebarRowFixture(14, "Maya", "direct", [7, 9])]));

    expect(startHuddleFromCommand(14, "Maya", join).kind).toBe("joining");
    expect(joins).toEqual([[14, "Maya", null]]);
  });

  it("passes a stage host's publish hint along", () => {
    const { joins, join } = recorder();
    const stage = sidebarRowFixture(20, "Town Hall", "stage");

    mutations.loadSidebar(
      sidebarFixture([{ ...stage, membership: { ...stage.membership, stageRole: "host" } }]),
    );

    startHuddleFromCommand(20, "Town Hall", join);

    expect(joins).toEqual([[20, "Town Hall", true]]);
  });

  it("refuses a board, which has no call button", () => {
    const { joins, join } = recorder();

    mutations.loadSidebar(sidebarFixture([sidebarRowFixture(30, "Roadmap", "board")]));

    expect(startHuddleFromCommand(30, "Roadmap", join)).toMatchObject({
      kind: "refused",
      title: "Boards don't have calls",
    });
    expect(joins).toEqual([]);
  });
});

describe("the composer's /huddle", () => {
  let network: MockNetwork;

  beforeAll(() => {
    network = installMockNetwork();

    const meta = document.createElement("meta");

    meta.name = "csrf-token";
    meta.content = network.server.csrfToken();
    document.head.append(meta);

    // jsdom doesn't lay out; the command list scrolls its active option into view.
    Element.prototype.scrollIntoView = () => undefined;
  });

  afterAll(() => network.restore());

  beforeEach(async () => {
    window.matchMedia = (query: string) =>
      Object.assign(new EventTarget(), {
        matches: false,
        media: query,
        onchange: null,
        addListener: () => undefined,
        removeListener: () => undefined,
      });

    // jsdom has no ResizeObserver; the send button's goo effect watches its size.
    vi.stubGlobal(
      "ResizeObserver",
      class {
        observe() {}
        unobserve() {}
        disconnect() {}
      },
    );

    const me: Me = await (await fetch("/api/v1/me")).json();

    mutations.reset();
    mutations.setMe(me);
    sessionStorage.clear();
  });

  afterEach(() => vi.restoreAllMocks());

  it("starts the room's call instead of pointing at the classic view", async () => {
    const user = userEvent.setup();
    const join = vi.spyOn(callController, "join").mockResolvedValue();

    render(<Composer roomId={ROOM} />);
    await user.type(screen.getByRole("textbox"), "/huddle{Enter}");

    await waitFor(() => expect(join).toHaveBeenCalledWith(ROOM, "general", null));
    expect(join).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("textbox")).toHaveProperty("value", "");
    expect(toastSnapshot().some((entry) => /classic|aren't in this app/.test(entry.title))).toBe(
      false,
    );
  });
});
