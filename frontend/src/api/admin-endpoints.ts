/**
 * The S7 admin endpoints (`/api/v1/admin/*`): the classic account pages. Everyone reads the
 * workspace and its people; everything else needs an administrator. Role changes, removal, custom
 * styles and a new join link fail with `SudoRequired` once the password confirmation has lapsed.
 */
import { Effect } from "effect";
import type { AuditLogFilters } from "../gen/AuditLogFilters.ts";
import type { AuditLogPage } from "../gen/AuditLogPage.ts";
import type { CreateIcon } from "../gen/CreateIcon.ts";
import type { CustomStyles } from "../gen/CustomStyles.ts";
import type { IntegrationsHealth } from "../gen/IntegrationsHealth.ts";
import type { PeoplePage } from "../gen/PeoplePage.ts";
import type { PersonChange } from "../gen/PersonChange.ts";
import type { PersonRemoved } from "../gen/PersonRemoved.ts";
import type { PersonRole } from "../gen/PersonRole.ts";
import type { UpdateWorkspace } from "../gen/UpdateWorkspace.ts";
import type { Workspace } from "../gen/Workspace.ts";
import type { WorkspaceIconList } from "../gen/WorkspaceIconList.ts";
import { call, get } from "./call.ts";
import {
  AuditLogPage as AuditLogPageSchema,
  CustomStyles as CustomStylesSchema,
  IntegrationsHealth as IntegrationsHealthSchema,
  PeoplePage as PeoplePageSchema,
  PersonChange as PersonChangeSchema,
  PersonRemoved as PersonRemovedSchema,
  WorkspaceIconList as WorkspaceIconListSchema,
  Workspace as WorkspaceSchema,
} from "./schema/admin.ts";
import { wire } from "./wire.ts";

const workspaceReply = wire<Workspace>(WorkspaceSchema);

/** `GET /admin/workspace`: the name, logo, join link and room-creation rule. */
export const workspace = Effect.fn("api.adminWorkspace")(function* () {
  return yield* call(get("/admin/workspace"), workspaceReply);
});

/** `PATCH /admin/workspace`: the name and the room-creation rule (`null` leaves one alone). */
export const updateWorkspace = Effect.fn("api.updateWorkspace")(function* (body: UpdateWorkspace) {
  return yield* call({ method: "PATCH", path: "/admin/workspace", body }, workspaceReply);
});

/** `PUT /admin/workspace/logo`: a finished direct upload becomes the logo. */
export const updateLogo = Effect.fn("api.updateLogo")(function* (signedId: string) {
  return yield* call(
    { method: "PUT", path: "/admin/workspace/logo", body: { signedId } },
    workspaceReply,
  );
});

/** `DELETE /admin/workspace/logo`: back to the default logo. */
export const removeLogo = Effect.fn("api.removeLogo")(function* () {
  return yield* call({ method: "DELETE", path: "/admin/workspace/logo" }, workspaceReply);
});

/** `PUT /admin/workspace/banner`: an uploaded image becomes the sidebar's banner. */
export const updateBanner = Effect.fn("api.updateBanner")(function* (signedId: string) {
  return yield* call(
    { method: "PUT", path: "/admin/workspace/banner", body: { signedId } },
    workspaceReply,
  );
});

/** `DELETE /admin/workspace/banner`: back to the plain header. */
export const removeBanner = Effect.fn("api.removeBanner")(function* () {
  return yield* call({ method: "DELETE", path: "/admin/workspace/banner" }, workspaceReply);
});

/** `POST /admin/workspace/join_code`: a new join link; the old one stops working. */
export const resetJoinCode = Effect.fn("api.resetJoinCode")(function* () {
  return yield* call({ method: "POST", path: "/admin/workspace/join_code" }, workspaceReply);
});

const peopleReply = wire<PeoplePage>(PeoplePageSchema);

/** `GET /admin/people`: administrators first, then members; `page` from `nextPage`. */
export const people = Effect.fn("api.adminPeople")(function* (page: string | null) {
  return yield* call(get("/admin/people", page === null ? undefined : { page }), peopleReply);
});

const changeReply = wire<PersonChange>(PersonChangeSchema);

/** `PATCH /admin/people/:id`: administrator or member. */
export const updatePerson = Effect.fn("api.updatePerson")(function* (
  userId: number,
  role: PersonRole,
) {
  return yield* call(
    { method: "PATCH", path: `/admin/people/${userId}`, body: { role } },
    changeReply,
  );
});

/** `DELETE /admin/people/:id`: removes them from the workspace for good. */
export const removePerson = Effect.fn("api.removePerson")(function* (userId: number) {
  return yield* call(
    { method: "DELETE", path: `/admin/people/${userId}` },
    wire<PersonRemoved>(PersonRemovedSchema),
  );
});

/** `POST /admin/people/:id/two_factor_reset`: they set two-step sign-in up again. */
export const resetTwoFactor = Effect.fn("api.resetTwoFactor")(function* (userId: number) {
  return yield* call(
    { method: "POST", path: `/admin/people/${userId}/two_factor_reset` },
    changeReply,
  );
});

/** `POST` (allow linking by email) or `DELETE` (unlink) `/admin/people/:id/google_link`. */
export const setGoogleLink = Effect.fn("api.setGoogleLink")(function* (
  userId: number,
  allow: boolean,
) {
  return yield* call(
    { method: allow ? "POST" : "DELETE", path: `/admin/people/${userId}/google_link` },
    changeReply,
  );
});

const stylesReply = wire<CustomStyles>(CustomStylesSchema);

/** `GET /admin/custom_styles`: the workspace's custom CSS. */
export const customStyles = Effect.fn("api.customStyles")(function* () {
  return yield* call(get("/admin/custom_styles"), stylesReply);
});

/** `PATCH /admin/custom_styles`: replaces it (`null` or blank clears it). */
export const updateCustomStyles = Effect.fn("api.updateCustomStyles")(function* (
  css: string | null,
) {
  return yield* call({ method: "PATCH", path: "/admin/custom_styles", body: { css } }, stylesReply);
});

const iconsReply = wire<WorkspaceIconList>(WorkspaceIconListSchema);

/** `GET /admin/icons`: the workspace's `:shortcode:` icons. */
export const icons = Effect.fn("api.workspaceIcons")(function* () {
  return yield* call(get("/admin/icons"), iconsReply);
});

/** `POST /admin/icons`: a new icon from a finished direct upload; answers the list. */
export const createIcon = Effect.fn("api.createIcon")(function* (body: CreateIcon) {
  return yield* call({ method: "POST", path: "/admin/icons", body }, iconsReply);
});

/** `DELETE /admin/icons/:id`: answers the list. */
export const destroyIcon = Effect.fn("api.destroyIcon")(function* (iconId: number) {
  return yield* call({ method: "DELETE", path: `/admin/icons/${iconId}` }, iconsReply);
});

/** The audit log's query: the filters that are set, and the page. */
export function auditLogQuery(filters: AuditLogFilters, page: string | null) {
  const query: Record<string, string> = {};

  const set = (key: string, value: string | null) => {
    if (value !== null && value.trim() !== "") {
      query[key] = value;
    }
  };

  set("actor", filters.actor);
  set("action", filters.action);
  set("targetType", filters.targetType);
  set("from", filters.from);
  set("to", filters.to);
  set("page", page);

  return query;
}

/** `GET /admin/audit_log`: newest first, filtered as the classic page filters. */
export const auditLog = Effect.fn("api.auditLog")(function* (
  filters: AuditLogFilters,
  page: string | null,
) {
  return yield* call(
    get("/admin/audit_log", auditLogQuery(filters, page)),
    wire<AuditLogPage>(AuditLogPageSchema),
  );
});

/** `GET /admin/integrations_health`: GitHub, Google, Fizzy, agent delivery and email. */
export const integrationsHealth = Effect.fn("api.integrationsHealth")(function* () {
  return yield* call(
    get("/admin/integrations_health"),
    wire<IntegrationsHealth>(IntegrationsHealthSchema),
  );
});
