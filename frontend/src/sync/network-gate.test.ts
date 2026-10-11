import { afterAll, beforeEach, expect, it, vi } from "vitest";
import { meFixture } from "../api/testing.ts";
import { mutations } from "../store/store.ts";
import { ignores } from "./admin.ts";
import { answeredForThisPage, pageReload } from "./sign-in-change.ts";

// The writes here come from a fresh API client: no CSRF token is held until the first one reads
// it. The tests run in order and share it, as one page would.

const B = meFixture.user.id + 1;

interface Sent {
  readonly method: string;
  readonly path: string;
  readonly reply: PromiseWithResolvers<Response>;
}

/** Every request that reached `fetch`, each answered when the test says. */
const sent: Sent[] = [];

// The action runtime keeps the `fetch` it first sees, so the stub stays for the whole file.
vi.spyOn(globalThis, "fetch").mockImplementation((input, init) => {
  const reply = Promise.withResolvers<Response>();

  sent.push({
    method: init?.method ?? "GET",
    path: new URL(String(input), "http://campfire.test").pathname,
    reply,
  });

  return reply.promise;
});

const reload = vi.spyOn(pageReload, "reload").mockImplementation(() => undefined);

/** Lets everything queued so far run. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 20));

/** Another account answers the page; the reload is held back. */
function anotherAccountFound() {
  expect(answeredForThisPage(B)).toBe(false);
  expect(reload).toHaveBeenCalledTimes(1);
}

beforeEach(() => {
  sent.length = 0;
  reload.mockClear();
  mutations.reset();
  mutations.setMe(meFixture);
});

afterAll(() => vi.restoreAllMocks());

it("sends no write once the first token read lands after another account was found", async () => {
  const write = ignores.set(5, true).catch((error: Error) => error);

  await settle();
  expect(sent.map((request) => `${request.method} ${request.path}`)).toEqual(["GET /api/v1/boot"]);

  anotherAccountFound();
  sent[0]?.reply.resolve(new Response(JSON.stringify({ csrfToken: "B's token" })));

  expect(await write).toBeInstanceOf(Error);
  expect(sent).toHaveLength(1);
});

it("sends nothing more when a write's stale-token 422 lands after another account was found", async () => {
  const write = ignores.set(5, true).catch((error: Error) => error);

  await settle();
  expect(sent.map((request) => `${request.method} ${request.path}`)).toEqual([
    "PUT /api/v1/people/5/block",
  ]);

  anotherAccountFound();
  // The CSRF check's bare 422: the client would read a fresh token, then send the write again.
  sent[0]?.reply.resolve(new Response("", { status: 422 }));

  expect(await write).toBeInstanceOf(Error);
  await settle();
  expect(sent).toHaveLength(1);
});
