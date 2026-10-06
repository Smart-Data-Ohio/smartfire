/**
 * Shared helpers for the S2 mock tests: a quiet server on a manual clock, typed requests, the
 * error summary of an `{"error": …}` body and a sync event collector. Test support only.
 */
import { expect } from "vitest";
import type { SyncEvent } from "../../src/gen/SyncEvent.ts";
import { field, type Json, stringField } from "../json.ts";
import { type ManualScheduler, manualScheduler } from "../scheduler.ts";
import { createMockServer, type MockResponse, type MockServer } from "../server.ts";

/** The tests' clock: a weekday afternoon in UTC. */
export const NOW = Date.UTC(2026, 9, 6, 16, 30, 0);

/** A server and the manual scheduler (and clock) it runs on. */
export interface Harness {
  readonly server: MockServer;
  readonly clock: ManualScheduler;
}

/** A seeded server with no simulation, on a manual clock starting at `NOW`. */
export function harness(seed = 1): Harness {
  const clock = manualScheduler(NOW);

  return { server: createMockServer({ now: () => clock.now(), seed, scheduler: clock }), clock };
}

/** `GET`, expecting 200, typed as the endpoint's reply. */
export async function get<T>(server: MockServer, path: string): Promise<T> {
  const response = await server.handle({ method: "GET", path });

  expect(response.status).toBe(200);

  // SAFETY: the mock builds these bodies from the generated wire types; the tests name which.
  return response.json as T;
}

/** A request with a body and the CSRF token. */
export function send(
  server: MockServer,
  method: string,
  path: string,
  body: Json = null,
): Promise<MockResponse> {
  return server.handle({
    method,
    path,
    body,
    headers: { "X-CSRF-Token": server.csrfToken() },
  });
}

/** A request expected to answer `status`, typed as the endpoint's reply. */
export async function expectStatus<T>(
  server: MockServer,
  method: string,
  path: string,
  body: Json,
  status: number,
): Promise<T> {
  const response = await send(server, method, path, body);

  expect(response.status, JSON.stringify(response.json)).toBe(status);

  // SAFETY: as in `get`, the test names the reply type of the endpoint it calls.
  return response.json as T;
}

/** The `_tag` and message of an `{"error": …}` body. */
export interface ErrorSummary {
  readonly tag: string | null;
  readonly message: string | null;
}

/** Summarizes an error body. */
export function errorOf(json: Json): ErrorSummary {
  const error = field(json, "error");

  return { tag: stringField(error, "_tag"), message: stringField(error, "message") };
}

/** Collects the events published on `topics` (plus `user`) from now on. */
export function collect(server: MockServer, topics: string[] = []): SyncEvent[] {
  const events: SyncEvent[] = [];

  const connection = server.connect((frame) => {
    if (frame.t === "batch") events.push(...frame.events);
  });

  connection.receive({ t: "hello", v: 1, resume: null, topics });

  return events;
}

/** A `CreateMessage` body. */
export function messageBody(
  clientMessageId: string,
  markdownSource: string,
  extra: Readonly<Record<string, Json>> = {},
): Json {
  return {
    clientMessageId,
    markdownSource,
    replyToMessageId: null,
    replyNotifyAuthor: null,
    ...extra,
  };
}
