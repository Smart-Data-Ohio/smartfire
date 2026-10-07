import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { CARD_IDS } from "../../../mock/s3/cards.ts";
import type { Me } from "../../gen/Me.ts";
import type { MessagePage } from "../../gen/MessagePage.ts";
import type { MessageDTO } from "../../store/model.ts";
import { mutations, store } from "../../store/store.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";

// This file imports the slot only inside its test, so the cards chunk hasn't arrived when the slot
// first renders: the case where the slot used to swap components when it landed.
let network: MockNetwork;

const ROOM = CARD_IDS.room;

function held(id: number): MessageDTO {
  const message = store.getState().messages[id];

  if (message === undefined) {
    throw new Error(`message ${id} isn't in the store`);
  }

  return message;
}

beforeAll(async () => {
  network = installMockNetwork();

  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });

  const me: Me = await (await fetch("/api/v1/me")).json();
  const page: MessagePage = await (await fetch(`/api/v1/rooms/${ROOM}/messages`)).json();

  mutations.reset();
  mutations.setMe(me);
  mutations.applyPage(ROOM, page, "replace");
});

afterAll(() => network.restore());

describe("the card slot", () => {
  it("keeps a card's local state when the cards chunk finishes loading", async () => {
    const user = userEvent.setup();
    const { CardSlot } = await import("./card-slot.tsx");
    const message = held(CARD_IDS.messages.pollMultiple);
    const view = render(<CardSlot message={message} threadId={null} />);

    // Suspended on the chunk at first.
    expect(view.container.textContent).toBe("");

    const poll = await screen.findByRole("region", { name: "Poll" }, { timeout: 10_000 });

    await user.click(within(poll).getByRole("button", { name: "Change vote" }));
    expect(within(poll).getByRole("button", { name: "Vote" })).toBeTruthy();

    // The row re-renders (a new copy of the message) now that the chunk is here.
    view.rerender(<CardSlot message={{ ...message }} threadId={null} />);

    const after = screen.getByRole("region", { name: "Poll" });

    expect(within(after).getByRole("button", { name: "Vote" })).toBeTruthy();
  });
});
