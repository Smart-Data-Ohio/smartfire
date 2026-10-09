import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
  useParams,
} from "@tanstack/react-router";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterAll, afterEach, beforeEach, describe, expect, it } from "vitest";
import { ok } from "../../mock/http.ts";
import {
  createMockServer,
  type MockRequest,
  type MockResponse,
  type MockServer,
} from "../../mock/server.ts";
import { meFixture, userFixture } from "../api/testing.ts";
import { MembersPane } from "../features/panes/members-pane.tsx";
import type { MemberList } from "../gen/MemberList.ts";
import { mutations } from "../store/store.ts";
import { installMockNetwork, type MockNetwork } from "../test/mock-network.ts";
import { invalidateRoom } from "./room-refresh.ts";

let network: MockNetwork | null = null;

let backend: MockServer | null = null;

const pendingReads: {
  request: MockRequest;
  reply: ReturnType<typeof Promise.withResolvers<MockResponse>>;
}[] = [];

const list = (userId: number): MemberList => ({
  members: [{ userId, presence: "online", statusText: null, starred: false }],
  users: [userFixture(userId, userId === 8 ? "Grace Hopper" : "Katherine Johnson")],
});

function holdMemberReads() {
  pendingReads.length = 0;

  if (network !== null) return pendingReads;

  const server = createMockServer();

  backend = server;
  network = installMockNetwork({
    ...server,
    handle(request) {
      if (request.method === "GET" && /\/rooms\/\d+\/members$/.test(request.path)) {
        const reply = Promise.withResolvers<MockResponse>();

        pendingReads.push({ request, reply });

        return reply.promise;
      }

      return server.handle(request);
    },
  });

  return pendingReads;
}

async function renderPane() {
  const root = createRootRoute({
    component: () => {
      const params = useParams({ strict: false });

      return <MembersPane roomId={Number(params.roomId)} />;
    },
  });

  const room = createRoute({ getParentRoute: () => root, path: "/r/$roomId" });

  const router = createRouter({
    routeTree: root.addChildren([room]),
    history: createMemoryHistory({ initialEntries: ["/r/12"] }),
  });

  const view = render(<RouterProvider router={router} />);

  await act(() => router.load());

  return { router, view };
}

beforeEach(() => {
  mutations.reset();
  mutations.setMe(meFixture);
});

afterEach(() => {
  cleanup();
  mutations.reset();
});

afterAll(() => {
  network?.restore();
  backend?.dispose();
  network = null;
  backend = null;
});

describe("mounted member list refresh", () => {
  it("rejects an older pending list after a management-triggered reload", async () => {
    const pending = holdMemberReads();

    await renderPane();
    await act(() => {
      invalidateRoom(12);
    });
    expect(pending).toHaveLength(2);
    await act(() => {
      pending[1]?.reply.resolve(ok(list(9)));
    });

    expect(screen.getByText("Katherine Johnson")).toBeDefined();
    await act(() => {
      pending[0]?.reply.resolve(ok(list(8)));
    });
    expect(screen.queryByText("Grace Hopper")).toBeNull();
    expect(screen.getByText("Katherine Johnson")).toBeDefined();
  });

  it("unsubscribes on room change and unmount, discarding prior-room completions", async () => {
    const pending = holdMemberReads();
    const { router, view } = await renderPane();

    await act(async () => {
      router.history.push("/r/13");
      await router.load();
    });
    act(() => {
      invalidateRoom(12);
    });
    expect(pending).toHaveLength(2);
    await act(() => {
      pending[1]?.reply.resolve(ok(list(9)));
      pending[0]?.reply.resolve(ok(list(8)));
    });
    expect(screen.queryByText("Grace Hopper")).toBeNull();
    expect(screen.getByText("Katherine Johnson")).toBeDefined();
    view.unmount();
    act(() => {
      invalidateRoom(13);
    });
    expect(pending).toHaveLength(2);
  });
});
