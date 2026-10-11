/**
 * The welcome card and server guide (U1, U4): everyone reads their own filtered copy at
 * `/welcome`; administrators read and replace the configuration at `/admin/welcome`.
 */
import { Effect } from "effect";
import type { WelcomeAdmin } from "../gen/WelcomeAdmin.ts";
import type { WelcomeSettings } from "../gen/WelcomeSettings.ts";
import type { WorkspaceWelcome } from "../gen/WorkspaceWelcome.ts";
import { call, get } from "./call.ts";
import {
  WelcomeAdmin as WelcomeAdminSchema,
  WorkspaceWelcome as WorkspaceWelcomeSchema,
} from "./schema/welcome.ts";
import { wire } from "./wire.ts";

const adminReply = wire<WelcomeAdmin>(WelcomeAdminSchema);

/** `GET /welcome`: the welcome card and guide, with only the rooms the viewer can reach. */
export const welcome = Effect.fn("api.welcome")(function* () {
  return yield* call(get("/welcome"), wire<WorkspaceWelcome>(WorkspaceWelcomeSchema));
});

/** `GET /admin/welcome`: the configuration and the rooms the editor offers. */
export const adminWelcome = Effect.fn("api.adminWelcome")(function* () {
  return yield* call(get("/admin/welcome"), adminReply);
});

/** `PUT /admin/welcome`: replaces the whole configuration. */
export const updateWelcome = Effect.fn("api.updateWelcome")(function* (body: WelcomeSettings) {
  return yield* call({ method: "PUT", path: "/admin/welcome", body }, adminReply);
});
