/**
 * Workspace administration (S7): the classic account pages' twin under `/admin/*`, kept per world
 * so `reset()` starts it over. The viewer is an administrator, as in the seed. People come from
 * the world's users (bots and deactivated people left out, banned people shown, as administrators
 * see them); the admin-only facts (email, two-step sign-in, Google) live here.
 */
import type { ApiError } from "../../src/gen/ApiError.ts";
import type { AuditLogEntry } from "../../src/gen/AuditLogEntry.ts";
import type { AuditLogPage } from "../../src/gen/AuditLogPage.ts";
import type { IntegrationsHealth } from "../../src/gen/IntegrationsHealth.ts";
import type { Person } from "../../src/gen/Person.ts";
import type { Workspace } from "../../src/gen/Workspace.ts";
import type { WorkspaceIcon } from "../../src/gen/WorkspaceIcon.ts";
import { HttpError, notFound, ok, plainError } from "../http.ts";
import { booleanField, type Json, stringField } from "../json.ts";
import { timestamp, VIEWER_ID, type World } from "../seed.ts";
import { firstId, type Route, route, type S2Context } from "./context.ts";
import type { Uploads } from "./uploads.ts";

/** Facts about a person only administrators see. */
interface Private {
  readonly emailAddress: string;
  twoFactorEnabled: boolean;
  googleIdentityEmail: string | null;
  offerGoogleEmailLink: boolean;
}

interface State {
  name: string;
  logoUrl: string | null;
  joinCode: string;
  restrict: boolean;
  css: string | null;
  icons: WorkspaceIcon[];
  nextIconId: number;
  removed: Set<number>;
  people: Map<number, Private>;
  audit: AuditLogEntry[];
  nextAuditId: number;
}

const ACTIONS = [
  "account.settings.change",
  "account.join_code.reset",
  "account.custom_styles.change",
  "user.role.change",
  "user.deactivate",
  "two_factor.reset",
  "workspace_icon.create",
  "workspace_icon.destroy",
  "google.sign_in.link_allow",
  "google.sign_in.unlink",
];

const TARGET_TYPES = ["Account", "User", "WorkspaceIcon"];

const ICON_NAME = /^[a-z0-9_]{2,32}$/;

/** The wire tag of a refused change (plain data here: Effect stays out of the mock). */
const VALIDATION: ApiError["_tag"] = "Validation";

/** The classic page's refusal, a 422 naming no field. */
const refusal = (message: string): HttpError =>
  new HttpError(422, { _tag: VALIDATION, message, fields: {} });

/** A 422 with the classic form's messages by field. */
const invalid = (fields: Readonly<Record<string, readonly string[]>>): HttpError =>
  new HttpError(422, {
    _tag: VALIDATION,
    message: `Validation failed: ${Object.values(fields).flat().join(", ")}`,
    fields: Object.fromEntries(Object.entries(fields).map(([key, value]) => [key, [...value]])),
  });

function initialState(world: World, now: number): State {
  const people = new Map<number, Private>();

  for (const user of world.users.values()) {
    const first = user.name.split(" ")[0]?.toLowerCase() ?? `user${user.id}`;

    people.set(user.id, {
      emailAddress: `${first}@smartdata.example`,
      twoFactorEnabled: user.id % 2 === 0,
      googleIdentityEmail: user.id === 3 ? `${first}@gmail.example` : null,
      offerGoogleEmailLink: user.id === 4,
    });
  }

  const viewer = world.users.get(VIEWER_ID)?.name ?? "You";

  return {
    name: "Smart Data",
    logoUrl: null,
    joinCode: "mock-join-code",
    restrict: false,
    css: null,
    icons: [
      {
        id: 1,
        name: "smartdata",
        title: "Smart Data",
        creatorName: viewer,
        imageUrl: "/icons/smartdata",
      },
    ],
    nextIconId: 2,
    removed: new Set(),
    people,
    audit: [
      {
        id: 1,
        createdAt: timestamp(now - 2 * 86_400_000),
        action: "account.settings.change",
        actor: viewer,
        target: "Smart Data",
        targetType: "Account",
        changes: "name: Campfire → Smart Data",
        ipAddress: "203.0.113.9",
      },
    ],
    nextAuditId: 2,
  };
}

/** The admin module. */
export interface AdminModule {
  readonly routes: readonly Route[];
  /** While on, the writes the classic pages guard with the password answer `SudoRequired`. */
  lapseSudo(on: boolean): void;
  /** Throws `SudoRequired` while the confirmation has lapsed, for the bot pages' guarded writes. */
  readonly requireSudo: () => void;
}

/** Creates the admin module. */
export function createAdmin(ctx: S2Context, uploads: Uploads): AdminModule {
  let state: State | null = null;
  let stateWorld: World | null = null;
  let sudoLapsed = false;

  /** `require_sudo_mode`: role changes, removal, custom styles and a new join link. */
  const requireSudo = () => {
    if (sudoLapsed) throw plainError(403, "SudoRequired", "Confirm your password to continue");
  };

  const current = (): State => {
    const world = ctx.world();

    if (state === null || stateWorld !== world) {
      state = initialState(world, ctx.now());
      stateWorld = world;
    }

    return state;
  };

  const viewerName = () => ctx.world().users.get(VIEWER_ID)?.name ?? "You";

  const audit = (action: string, target: string | null, targetType: string, changes: string) => {
    const held = current();

    held.audit.unshift({
      id: held.nextAuditId,
      createdAt: timestamp(ctx.now()),
      action,
      actor: viewerName(),
      target,
      targetType,
      changes,
      ipAddress: "203.0.113.9",
    });
    held.nextAuditId += 1;
  };

  const workspace = (): Workspace => {
    const held = current();

    return {
      name: held.name,
      logoUrl: held.logoUrl ?? "/account/logo",
      logoAttached: held.logoUrl !== null,
      joinUrl: `http://127.0.0.1/join/${held.joinCode}`,
      canAdminister: true,
      restrictRoomCreationToAdministrators: held.restrict,
      version: "2.0.0-mock",
    };
  };

  const person = (id: number): Person | null => {
    const held = current();
    const user = ctx.world().users.get(id);
    const facts = held.people.get(id);

    if (
      user === undefined ||
      facts === undefined ||
      held.removed.has(id) ||
      user.role === "bot" ||
      user.status === "deactivated"
    ) {
      return null;
    }

    return {
      id,
      name: user.name,
      avatarUrl: user.avatarUrl,
      role: user.role === "administrator" ? "administrator" : "member",
      banned: user.status === "banned",
      you: id === VIEWER_ID,
      twoFactorEnabled: facts.twoFactorEnabled,
      emailAddress: facts.emailAddress,
      googleIdentityEmail: facts.googleIdentityEmail,
      offerGoogleEmailLink: facts.googleIdentityEmail === null && facts.offerGoogleEmailLink,
    };
  };

  const personOr404 = (id: number): Person => {
    const found = person(id);

    if (found === null) throw notFound();

    return found;
  };

  const people = (): Person[] =>
    [...ctx.world().users.keys()]
      .flatMap((id) => {
        const found = person(id);

        return found === null ? [] : [found];
      })
      .sort((a, b) =>
        a.role === b.role ? a.name.localeCompare(b.name) : a.role === "administrator" ? -1 : 1,
      );

  const updateWorkspace = (body: Json | undefined) => {
    const held = current();
    const name = stringField(body, "name");

    const restrict = booleanField(body, "restrictRoomCreationToAdministrators");

    if (name !== null && name.trim() !== "" && name !== held.name) {
      audit("account.settings.change", name, "Account", `name: ${held.name} → ${name}`);
      held.name = name;
    }

    if (restrict !== null && restrict !== held.restrict) {
      audit(
        "account.settings.change",
        held.name,
        "Account",
        `restrict_room_creation_to_administrators: ${held.restrict} → ${restrict}`,
      );
      held.restrict = restrict;
    }

    return ok(workspace());
  };

  const setRole = (id: number, body: Json | undefined) => {
    const target = personOr404(id);

    requireSudo();
    const role = stringField(body, "role");
    const users = ctx.world().users;
    const user = users.get(id);

    if (user !== undefined && !target.you && (role === "administrator" || role === "member")) {
      if (role !== target.role) {
        audit("user.role.change", user.name, "User", `role: ${target.role} → ${role}`);
      }

      users.set(id, { ...user, role, updatedAt: new Date(ctx.now()).toISOString() });
    }

    return ok({ person: personOr404(id), notice: null });
  };

  const remove = (id: number) => {
    const target = personOr404(id);

    requireSudo();
    current().removed.add(id);
    audit("user.deactivate", target.name, "User", "");

    return ok({ id });
  };

  const resetTwoFactor = (id: number) => {
    const target = personOr404(id);
    const facts = current().people.get(id);

    if (target.you) {
      throw refusal(
        "Reset someone else's two-step sign-in from here. To change your own, use Disable on your profile.",
      );
    }

    if (facts === undefined || !facts.twoFactorEnabled) {
      throw refusal(`${target.name} doesn't have two-step sign-in enabled.`);
    }

    facts.twoFactorEnabled = false;
    audit("two_factor.reset", target.name, "User", "");

    return ok({
      person: personOr404(id),
      notice: `Two-step sign-in reset for ${target.name}. They will set it up again at next sign-in.`,
    });
  };

  const googleLink = (id: number, allow: boolean) => {
    const target = personOr404(id);
    const facts = current().people.get(id);

    if (facts !== undefined) {
      if (allow) {
        facts.offerGoogleEmailLink = false;
        audit("google.sign_in.link_allow", target.name, "User", "");
      } else if (facts.googleIdentityEmail !== null) {
        facts.googleIdentityEmail = null;
        audit("google.sign_in.unlink", target.name, "User", "");
      }
    }

    return ok({
      person: personOr404(id),
      notice: allow
        ? `${target.name} can now link Google sign-in for ${target.emailAddress ?? ""}.`
        : `Google sign-in unlinked from ${target.name}.`,
    });
  };

  const updateStyles = (body: Json | undefined) => {
    requireSudo();

    const held = current();
    const css = stringField(body, "css");
    const next = css === null || css.trim() === "" ? null : css;

    if (next !== held.css) {
      audit("account.custom_styles.change", held.name, "Account", "custom_styles changed");
      held.css = next;
    }

    return ok({ css: held.css });
  };

  const icons = () => ok({ icons: current().icons });

  const createIcon = (body: Json | undefined) => {
    const held = current();
    const name = (stringField(body, "name") ?? "").trim().toLowerCase();
    const title = (stringField(body, "title") ?? "").trim();
    const signedId = stringField(body, "signedId");
    const fields: Record<string, string[]> = {};

    if (!ICON_NAME.test(name)) {
      fields.name = ["Name must be 2–32 lowercase letters, numbers or underscores"];
    } else if (held.icons.some((icon) => icon.name === name)) {
      fields.name = ["Name has already been taken"];
    }

    if (title === "") fields.title = ["Title can't be blank"];

    if (signedId === null) fields.image = ["Image must be attached"];

    if (Object.keys(fields).length > 0) throw invalid(fields);

    if (signedId !== null) uploads.attachment(signedId);

    held.icons = [
      ...held.icons,
      {
        id: held.nextIconId,
        name,
        title,
        creatorName: viewerName(),
        imageUrl: `/icons/${name}`,
      },
    ].sort((a, b) => a.name.localeCompare(b.name));
    held.nextIconId += 1;
    audit("workspace_icon.create", `:${name}:`, "WorkspaceIcon", `name: ${name}, title: ${title}`);

    return icons();
  };

  const destroyIcon = (id: number) => {
    const held = current();
    const icon = held.icons.find((each) => each.id === id);

    if (icon === undefined) throw notFound();

    held.icons = held.icons.filter((each) => each.id !== id);
    audit("workspace_icon.destroy", `:${icon.name}:`, "WorkspaceIcon", `name: ${icon.name}`);

    return icons();
  };

  const auditLog = (query: URLSearchParams): AuditLogPage => {
    const value = (key: string) => {
      const raw = query.get(key);

      return raw === null || raw.trim() === "" ? null : raw.trim();
    };

    const filters = {
      actor: value("actor"),
      action: value("action"),
      targetType: value("targetType"),
      from: value("from"),
      to: value("to"),
    };

    const entries = current().audit.filter(
      (entry) =>
        (filters.actor === null ||
          (entry.actor ?? "").toLowerCase().includes(filters.actor.toLowerCase())) &&
        (filters.action === null || entry.action === filters.action) &&
        (filters.targetType === null || entry.targetType === filters.targetType) &&
        (filters.from === null || entry.createdAt.slice(0, 10) >= filters.from) &&
        (filters.to === null || entry.createdAt.slice(0, 10) <= filters.to),
    );

    const exportQuery = new URLSearchParams(
      Object.entries(filters).flatMap(([key, each]) => (each === null ? [] : [[key, each]])),
    ).toString();

    return {
      filters,
      entries,
      nextPage: null,
      actions: ACTIONS,
      targetTypes: TARGET_TYPES,
      exportUrl: `/account/audit_log.csv${exportQuery === "" ? "" : `?${exportQuery}`}`,
      exportTruncated: false,
      exportLimit: 5000,
      timeZone: "America/New_York",
    };
  };

  const health = (): IntegrationsHealth => ({
    github: {
      workspaceToken: true,
      appConfigured: true,
      webhookSecret: false,
      connected: 3,
      appTokens: 1,
      deliveries24h: 12,
      disconnected: [{ subject: "@jonah", detail: "Bad credentials" }],
      lastErrors: [],
      fetchErrors: [],
    },
    google: {
      configured: true,
      connected: 2,
      pushEnabled: false,
      pushChannels: 0,
      disconnected: [],
      entryErrors: [],
      expiring: [],
    },
    fizzy: { configured: false, note: "No Fizzy integration is configured in this workspace." },
    agentDelivery: { pending: 0, failed24h: 0, recentErrors: [] },
    email: { enabled: true, roomsWithAddresses: 1 },
  });

  return {
    lapseSudo: (on) => {
      sudoLapsed = on;
    },
    requireSudo,
    routes: [
      route("GET", /^\/admin\/workspace$/, () => ok(workspace())),
      route("PATCH", /^\/admin\/workspace$/, ({ body }) => updateWorkspace(body)),
      route("PUT", /^\/admin\/workspace\/logo$/, ({ body }) => {
        const signedId = stringField(body, "signedId");

        if (signedId === null) throw invalid({ signedId: ["isn't an uploaded file"] });

        current().logoUrl = uploads.attachment(signedId).url;

        return ok(workspace());
      }),
      route("DELETE", /^\/admin\/workspace\/logo$/, () => {
        current().logoUrl = null;

        return ok(workspace());
      }),
      route("POST", /^\/admin\/workspace\/join_code$/, () => {
        requireSudo();
        current().joinCode = ctx.hex(12);
        audit("account.join_code.reset", current().name, "Account", "");

        return ok(workspace());
      }),
      route("GET", /^\/admin\/people$/, () => ok({ people: people(), nextPage: null })),
      route("PATCH", /^\/admin\/people\/(\d+)$/, (request) =>
        setRole(firstId(request), request.body),
      ),
      route("DELETE", /^\/admin\/people\/(\d+)$/, (request) => remove(firstId(request))),
      route("POST", /^\/admin\/people\/(\d+)\/two_factor_reset$/, (request) =>
        resetTwoFactor(firstId(request)),
      ),
      route("POST", /^\/admin\/people\/(\d+)\/google_link$/, (request) =>
        googleLink(firstId(request), true),
      ),
      route("DELETE", /^\/admin\/people\/(\d+)\/google_link$/, (request) =>
        googleLink(firstId(request), false),
      ),
      route("GET", /^\/admin\/custom_styles$/, () => ok({ css: current().css })),
      route("PATCH", /^\/admin\/custom_styles$/, ({ body }) => updateStyles(body)),
      route("GET", /^\/admin\/icons$/, () => icons()),
      route("POST", /^\/admin\/icons$/, ({ body }) => createIcon(body)),
      route("DELETE", /^\/admin\/icons\/(\d+)$/, (request) => destroyIcon(firstId(request))),
      route("GET", /^\/admin\/audit_log$/, ({ query }) => ok(auditLog(query))),
      route("GET", /^\/admin\/integrations_health$/, () => ok(health())),
    ],
  };
}
