/**
 * The viewer's settings (S7): the classic profile page's sections, sessions and push
 * subscriptions, kept per world so `reset()` starts them over. DND exceptions live in their own
 * set, apart from the stars, as the server keeps them in their own table.
 */

import type { CreatePushSubscription } from "../../src/gen/CreatePushSubscription.ts";
import type { IntegrationSettings } from "../../src/gen/IntegrationSettings.ts";
import type { NotificationLevel } from "../../src/gen/NotificationLevel.ts";
import type { PushPublicKey } from "../../src/gen/PushPublicKey.ts";
import type { PushSubscriptionList } from "../../src/gen/PushSubscriptionList.ts";
import type { SessionInfo } from "../../src/gen/SessionInfo.ts";
import type { SessionList } from "../../src/gen/SessionList.ts";
import type { Settings } from "../../src/gen/Settings.ts";
import type { StatusExpiry } from "../../src/gen/StatusExpiry.ts";
import type { TextSize } from "../../src/gen/TextSize.ts";
import type { Theme } from "../../src/gen/Theme.ts";
import { noContent, notFound, ok, plainError, refused, validation } from "../http.ts";
import {
  booleanField,
  field,
  intField,
  isBoolean,
  isRecord,
  type Json,
  stringArrayField,
  stringField,
} from "../json.ts";
import { rowTimestamp, timestamp, VIEWER_ID, type World } from "../seed.ts";
import { VIEWER_TIME_ZONE } from "./composer.ts";
import { firstId, type Route, route, type S2Context } from "./context.ts";
import type { Uploads } from "./uploads.ts";

const MINUTE_MS = 60_000;

const HOUR_MS = 60 * MINUTE_MS;

const DAY_MS = 24 * HOUR_MS;

/** The viewer's password in the mock, for the email change check. */
export const MOCK_PASSWORD = "secret123456";

/** A public P-256 test key; the mock never sends real push notifications. */
export const MOCK_PUSH_PUBLIC_KEY =
  "BEYXTBB5_jNhNzXDmx5KEU55Vbbd-u--Lk9rM5OFQvUkPIBwZJ9QzAq0zdEzFw6yTV8cTriz_qYBVicY02_VxTQ=";

/** The time zone choices the mock offers (the server lists every zone). */
const TIME_ZONES = [
  { label: "(GMT-08:00) Pacific Time (US & Canada)", value: "America/Los_Angeles" },
  { label: "(GMT-05:00) Eastern Time (US & Canada)", value: "America/New_York" },
  { label: "(GMT+00:00) London", value: "Europe/London" },
  { label: "(GMT+01:00) Berlin", value: "Europe/Berlin" },
  { label: "(GMT+09:00) Tokyo", value: "Asia/Tokyo" },
];

const INBOX = [
  {
    key: "mentions",
    label: "Mentions",
    description: "When someone mentions you or a group you belong to.",
  },
  {
    key: "github_review_requests",
    label: "GitHub review requests",
    description: "When someone asks you to review a pull request.",
  },
  {
    key: "agent_work",
    label: "Agent work",
    description: "When an agent finishes or needs you on a task you started.",
  },
];

/** How long each "Clear after" choice lasts (the mock's "today" is eight hours). */
const EXPIRY_MS = new Map<StatusExpiry, number | null>([
  ["minutes_30", 30 * MINUTE_MS],
  ["hour_1", HOUR_MS],
  ["hours_4", 4 * HOUR_MS],
  ["today", 8 * HOUR_MS],
  ["week", 7 * DAY_MS],
  ["never", null],
]);

/** What the mock keeps beyond the world's users and stars. */
interface State {
  settings: Settings;
  sessions: SessionInfo[];
  push: PushSubscriptionList;
  pushKeys: Map<number, CreatePushSubscription>;
}

function initialState(world: World, now: number): State {
  const viewer = world.users.get(VIEWER_ID);

  return {
    settings: {
      revision: 0,
      evaluatedAt: new Date(now).toISOString().replace("Z", "000000Z"),
      profile: {
        userId: VIEWER_ID,
        name: viewer?.name ?? "You",
        emailAddress: "riel@smartdata.example",
        bio: viewer?.bio ?? null,
        avatarUrl: viewer?.avatarUrl ?? "/avatar.svg",
        avatarAttached: false,
        hasPassword: true,
        githubLogin: "riel",
        githubVerified: false,
        bot: false,
      },
      appearance: {
        theme: "system",
        textSize: "default",
        timeZone: VIEWER_TIME_ZONE,
        timeZones: TIME_ZONES,
      },
      notifications: {
        defaultNotificationLevel: "everything",
        roomNotificationLevels: {},
        roomMuteUntil: {},
        dndEnabled: false,
        quietHoursEnabled: false,
        quietHoursStart: "22:00",
        quietHoursEnd: "07:00",
        meetingDndEnabled: false,
        oooNotifyEnabled: true,
        allowedPeople: [],
        keywordAlerts: ["deploy freeze"],
        inbox: INBOX.map((entry) => ({ ...entry, enabled: entry.key !== "agent_work" })),
      },
      status: {
        presenceSetting: "auto",
        customStatusEmoji: null,
        customStatusText: null,
        customStatusExpiresAt: null,
        meetingStatusEnabled: true,
        oooCalendarEnabled: false,
        oooUntil: null,
        oooManual: false,
        oooNote: null,
        calendarError: null,
      },
      calls: { voiceMode: "voice_activity", pushToTalkKey: "`" },
      integrations: {
        google: {
          signInConfigured: true,
          identityEmail: null,
          calendarConfigured: true,
          connected: true,
          calendar: true,
          drive: false,
          email: "riel@smartdata.example",
        },
        github: { state: "connected", name: "riel", workspace: null, appToken: false },
        githubAppConfigured: true,
        fizzy: { state: "missing" },
        managePath: "/users/me/profile",
        slackImportPath: "/slack/imports",
      },
    },
    sessions: [
      {
        id: 3,
        current: true,
        description: "Firefox on Linux",
        ipAddress: "203.0.113.9",
        lastActiveAt: timestamp(now - MINUTE_MS),
        createdAt: timestamp(now - 5 * DAY_MS),
      },
      {
        id: 2,
        current: false,
        description: "Safari on iPhone",
        ipAddress: "198.51.100.4",
        lastActiveAt: timestamp(now - 3 * HOUR_MS),
        createdAt: timestamp(now - 20 * DAY_MS),
      },
      {
        id: 1,
        current: false,
        description: "Chrome on macOS",
        ipAddress: null,
        lastActiveAt: timestamp(now - 2 * DAY_MS),
        createdAt: timestamp(now - 40 * DAY_MS),
      },
    ],
    push: {
      pushSubscriptions: [
        {
          id: 4,
          endpoint: "https://updates.push.services.mozilla.com/wpush/v2/gAAAAABmock",
          browser: "Firefox",
          version: "143",
          platform: "Linux",
        },
        {
          id: 5,
          endpoint: "https://web.push.apple.com/QmockiPhone",
          browser: "Safari",
          version: "26",
          platform: "iOS",
        },
      ],
    },
    pushKeys: new Map(),
  };
}

/** The settings module. */
export interface SettingsModule {
  readonly routes: readonly Route[];
  /** The viewer's saved theme and text size, which boot and `/me` carry. */
  readonly appearance: () => { readonly theme: Theme; readonly textSize: TextSize };
}

/** A value of `body[key]` when the key is present and not null. */
function given(body: Json | undefined, key: string): Json | undefined {
  const value = field(body, key);

  return value === null ? undefined : value;
}

/** A text setting after a write: unchanged for `null`, cleared for "", else the new value. */
function changedTo(next: string | null, held: string | null): string | null {
  if (next === null) return held;

  return next === "" ? null : next;
}

/** `sessions#revoke_others`'s notice. */
function signedOutNotice(count: number): string {
  if (count === 0) return "No other sessions to sign out.";

  if (count === 1) return "Signed out 1 other session.";

  return `Signed out ${count} other sessions.`;
}

/** Creates the settings module. */
export function createSettings(
  ctx: S2Context,
  uploads: Uploads,
  requireSudo: () => void,
): SettingsModule {
  let state: State | null = null;
  let stateWorld: World | null = null;

  const current = (): State => {
    const world = ctx.world();

    if (state === null || stateWorld !== world) {
      state = initialState(world, ctx.now());
      stateWorld = world;
    }

    return state;
  };

  /** The settings page, with the viewer's DND exceptions. */
  const page = (): Settings => {
    const world = ctx.world();
    const now = ctx.now();
    const { settings } = current();

    const allowedPeople = [...world.dndAllowed]
      .flatMap((id) => {
        const user = world.users.get(id);

        return user === undefined ? [] : [{ userId: user.id, name: user.name }];
      })
      .sort((a, b) => a.name.localeCompare(b.name));

    return {
      ...settings,
      evaluatedAt: new Date(now).toISOString().replace("Z", "000000Z"),
      notifications: {
        ...settings.notifications,
        allowedPeople,
        roomMuteUntil: Object.fromEntries(
          Object.entries(settings.notifications.roomMuteUntil).filter(
            ([, until]) => until === null || Date.parse(until) > now,
          ),
        ),
      },
    };
  };

  const update = (change: (settings: Settings) => Settings) => {
    const held = current();

    held.settings = { ...change(held.settings), revision: held.settings.revision + 1 };
    ctx.world().activityRevision++;

    return ok(page());
  };

  const renameViewer = (name: string) => {
    const users = ctx.world().users;
    const viewer = users.get(VIEWER_ID);

    if (viewer !== undefined) {
      users.set(VIEWER_ID, { ...viewer, name, updatedAt: rowTimestamp(ctx.now()) });
    }
  };

  const profile = (body: Json | undefined) => {
    const { settings } = current();
    const name = stringField(body, "name");
    const email = stringField(body, "emailAddress");

    if (name !== null && name.trim() === "") throw validation("name", "can't be blank");

    if (email !== null && email !== settings.profile.emailAddress) {
      const password = stringField(body, "currentPassword");

      if (password === null || password === "") {
        throw validation("currentPassword", "is required to change your email address");
      }

      if (password !== MOCK_PASSWORD) throw validation("currentPassword", "is incorrect");

      if (!email.includes("@")) throw validation("emailAddress", "is invalid");
    }

    const bio = stringField(body, "bio");

    if (bio !== null && bio.length > 200) throw validation("bio", "is too long");

    if (name !== null) renameViewer(name);

    const github = stringField(body, "githubLogin");

    return update((held) => ({
      ...held,
      profile: {
        ...held.profile,
        name: name ?? held.profile.name,
        emailAddress: email ?? held.profile.emailAddress,
        bio: changedTo(bio, held.profile.bio),
        githubLogin: held.profile.githubVerified
          ? held.profile.githubLogin
          : changedTo(github, held.profile.githubLogin),
      },
    }));
  };

  const avatar = (signedId: string | null) =>
    update((held) => ({
      ...held,
      profile: {
        ...held.profile,
        avatarUrl:
          signedId === null
            ? (ctx.world().users.get(VIEWER_ID)?.avatarUrl ?? held.profile.avatarUrl)
            : uploads.attachment(signedId).url,
        avatarAttached: signedId !== null,
      },
    }));

  const appearance = (body: Json | undefined) =>
    update((held) => {
      const theme = stringField(body, "theme");
      const textSize = stringField(body, "textSize");
      const zone = stringField(body, "timeZone");
      const themes = ["system", "light", "dark"] as const;
      const sizes = ["smaller", "small", "default", "large", "larger"] as const;

      return {
        ...held,
        appearance: {
          ...held.appearance,
          theme: themes.find((value) => value === theme) ?? held.appearance.theme,
          textSize: sizes.find((value) => value === textSize) ?? held.appearance.textSize,
          timeZone: changedTo(zone, held.appearance.timeZone),
        },
      };
    });

  const calls = (body: Json | undefined) => {
    const key = stringField(body, "pushToTalkKey");

    if (key !== null && key.length > 20) {
      throw validation("pushToTalkKey", "is too long (maximum is 20 characters)");
    }

    return update((held) => {
      const mode = stringField(body, "voiceMode");

      return {
        ...held,
        calls: {
          voiceMode:
            mode === "push_to_talk" || mode === "voice_activity" ? mode : held.calls.voiceMode,
          pushToTalkKey: key ?? held.calls.pushToTalkKey,
        },
      };
    });
  };

  const notificationLevel = (value: Json | undefined): NotificationLevel => {
    if (value === "everything" || value === "mentions" || value === "nothing") return value;
    throw validation("defaultNotificationLevel", "is invalid");
  };

  const notifications = (body: Json | undefined) => {
    const keywords = stringArrayField(body, "keywordAlerts");

    if (keywords !== null && keywords.length > 20) {
      throw validation("keywordAlerts", "can't have more than 20 keywords");
    }

    const inbox = given(body, "inbox");

    return update((held) => {
      const flag = (key: string, fallback: boolean) => booleanField(body, key) ?? fallback;
      const n = held.notifications;
      const defaultLevel = given(body, "defaultNotificationLevel");
      const roomNotification = given(body, "roomNotification");
      const roomMute = given(body, "roomMute");
      const roomNotificationLevels = { ...n.roomNotificationLevels };
      const roomMuteUntil = { ...n.roomMuteUntil };

      if (isRecord(roomNotification)) {
        const roomId = intField(roomNotification, "roomId");

        if (roomId === null || !ctx.world().rooms.has(roomId)) throw notFound("Room");
        roomNotificationLevels[String(roomId)] =
          roomNotification.level === null ? null : notificationLevel(roomNotification.level);
      }

      if (isRecord(roomMute)) {
        const roomId = intField(roomMute, "roomId");

        if (roomId === null || !ctx.world().rooms.has(roomId)) throw notFound("Room");
        const duration = stringField(roomMute, "duration");

        if (duration === "off") delete roomMuteUntil[String(roomId)];
        else if (duration === "forever") roomMuteUntil[String(roomId)] = null;
        else {
          const seconds = new Map<string, number>([
            ["minutes15", 900],
            ["hour1", 3600],
            ["hours8", 28800],
            ["hours24", 86400],
          ]).get(duration ?? "");

          if (seconds === undefined) throw validation("roomMute", "is invalid");
          roomMuteUntil[String(roomId)] = new Date(ctx.now() + seconds * 1000).toISOString();
        }
      }

      return {
        ...held,
        notifications: {
          ...n,
          defaultNotificationLevel:
            defaultLevel === undefined || defaultLevel === null
              ? n.defaultNotificationLevel
              : notificationLevel(defaultLevel),
          roomNotificationLevels,
          roomMuteUntil,
          dndEnabled: flag("dndEnabled", n.dndEnabled),
          quietHoursEnabled: flag("quietHoursEnabled", n.quietHoursEnabled),
          quietHoursStart: stringField(body, "quietHoursStart") ?? n.quietHoursStart,
          quietHoursEnd: stringField(body, "quietHoursEnd") ?? n.quietHoursEnd,
          meetingDndEnabled: flag("meetingDndEnabled", n.meetingDndEnabled),
          oooNotifyEnabled: flag("oooNotifyEnabled", n.oooNotifyEnabled),
          keywordAlerts: keywords ?? n.keywordAlerts,
          inbox: n.inbox.map((entry) => {
            const value = isRecord(inbox) ? inbox[entry.key] : undefined;

            return isBoolean(value) ? { ...entry, enabled: value } : entry;
          }),
        },
      };
    });
  };

  /** The classic out-of-office presets, from the mock's clock. */
  const oooEnd = (preset: string, custom: string | null): number => {
    const now = ctx.now();

    if (preset === "tomorrow") return now + DAY_MS;

    if (preset === "monday") return now + ((8 - new Date(now).getUTCDay()) % 7 || 7) * DAY_MS;

    if (preset === "week") return now + 7 * DAY_MS;

    const at = custom === null ? Number.NaN : Date.parse(`${custom}Z`);

    if (!Number.isFinite(at) || at <= now) {
      throw validation("oooUntil", "needs a future date and time");
    }

    return at;
  };

  const status = (body: Json | undefined) => {
    const preset = stringField(body, "oooPreset");
    const until = preset === null ? null : oooEnd(preset, stringField(body, "oooUntilCustom"));

    return update((held) => {
      let next = { ...held.status };
      const presence = stringField(body, "presenceSetting");

      if (presence === "auto" || presence === "dnd" || presence === "invisible") {
        next.presenceSetting = presence;
      }

      if (booleanField(body, "clearCustomStatus") === true) {
        next = {
          ...next,
          customStatusEmoji: null,
          customStatusText: null,
          customStatusExpiresAt: null,
        };
      } else if (
        given(body, "customStatusText") !== undefined ||
        given(body, "customStatusEmoji") !== undefined
      ) {
        const text = stringField(body, "customStatusText") ?? "";
        const emoji = stringField(body, "customStatusEmoji") ?? "";
        const expiry = stringField(body, "customStatusExpiresIn");
        const ms = [...EXPIRY_MS].find(([key]) => key === expiry)?.[1] ?? null;

        next = {
          ...next,
          customStatusText: text === "" ? null : text,
          customStatusEmoji: emoji === "" ? null : emoji,
          customStatusExpiresAt:
            ms === null || (text === "" && emoji === "") ? null : timestamp(ctx.now() + ms),
        };
      }

      const meeting = booleanField(body, "meetingStatusEnabled");
      const calendar = booleanField(body, "oooCalendarEnabled");

      if (meeting !== null) next.meetingStatusEnabled = meeting;

      if (calendar !== null) next.oooCalendarEnabled = calendar;

      if (booleanField(body, "clearOoo") === true) {
        next = { ...next, oooUntil: null, oooManual: false, oooNote: null };
      } else {
        if (until !== null) next = { ...next, oooUntil: timestamp(until), oooManual: true };

        const note = stringField(body, "oooNote");

        if (note !== null) next.oooNote = note === "" ? null : note;
      }

      return { ...held, status: next };
    });
  };

  const allowance = (userId: number, allowed: boolean) => {
    const world = ctx.world();
    const user = world.users.get(userId);

    if (user === undefined || user.status !== "active" || user.role === "bot") {
      throw notFound("User not found");
    }

    if (userId === VIEWER_ID) throw validation("userId", "can't be you");

    if (allowed) world.dndAllowed.add(userId);
    else world.dndAllowed.delete(userId);

    return update((held) => held);
  };

  const sessionList = (notice: string | null): SessionList => ({
    sessions: current().sessions,
    notice,
  });

  const revoke = (id: number) => {
    const held = current();
    const session = held.sessions.find((candidate) => candidate.id === id);

    if (session === undefined) throw notFound();

    if (session.current) throw plainError(401, "Unauthorized", "Signed out");

    held.sessions = held.sessions.filter((candidate) => candidate.id !== id);

    return ok(sessionList("Signed out that session."));
  };

  const revokeOthers = () => {
    const held = current();
    const count = held.sessions.filter((session) => !session.current).length;

    held.sessions = held.sessions.filter((session) => session.current);

    return ok(sessionList(signedOutNotice(count)));
  };

  const removePush = (id: number) => {
    const held = current();

    held.pushKeys.delete(id);

    held.push = {
      pushSubscriptions: held.push.pushSubscriptions.filter(
        (subscription) => subscription.id !== id,
      ),
    };

    return ok(held.push);
  };

  const createPush = (body: Json | undefined) => {
    const endpoint = stringField(body, "endpoint");
    const p256dhKey = stringField(body, "p256dhKey");
    const authKey = stringField(body, "authKey");

    if (endpoint === null || p256dhKey === null || authKey === null) {
      throw validation("endpoint", "The push endpoint and keys must be strings.");
    }

    let url: URL;

    try {
      url = new URL(endpoint);
    } catch {
      throw validation("endpoint", "The push endpoint is not valid.");
    }

    const permitted = [
      "jmt17.google.com",
      "fcm.googleapis.com",
      "updates.push.services.mozilla.com",
      "web.push.apple.com",
      "notify.windows.com",
    ];

    if (
      url.protocol !== "https:" ||
      url.port !== "" ||
      !permitted.some((host) => url.hostname === host || url.hostname.endsWith(`.${host}`))
    ) {
      throw validation("endpoint", "The push endpoint is not a permitted push service.");
    }

    const held = current();

    const existing = [...held.pushKeys].find(
      ([, keys]) =>
        keys.endpoint === endpoint && keys.p256dhKey === p256dhKey && keys.authKey === authKey,
    );

    if (existing !== undefined) return ok(held.push);

    const id =
      Math.max(0, ...held.push.pushSubscriptions.map((subscription) => subscription.id)) + 1;

    held.pushKeys.set(id, { endpoint, p256dhKey, authKey });
    held.push = {
      pushSubscriptions: [
        ...held.push.pushSubscriptions,
        { id, endpoint, browser: "Chrome", version: "144", platform: "Linux" },
      ],
    };

    return ok(held.push);
  };

  /**
   * A connect or disconnect, as the classic profile's: the password confirmation first, then the
   * change and its notice. `change` may refuse with the classic alert.
   */
  const integration = (
    change: (held: Settings) => { readonly settings: Settings; readonly notice: string },
  ) => {
    requireSudo();

    const held = current();
    const next = change(held.settings);
    const integrations: IntegrationSettings = next.settings.integrations;

    held.settings = next.settings;

    return ok({ integrations, notice: next.notice });
  };

  /**
   * A pasted token as the services answer it in the mock: blank is refused, `bad…` is rejected,
   * `offline…` can't be reached and (Fizzy) `none…` has no account; anything else connects.
   */
  const checkToken = (service: "GitHub" | "Fizzy", body: Json | undefined): void => {
    const token = (stringField(body, "accessToken") ?? "").trim();

    if (token === "") throw refused(`Paste a token to connect ${service}.`);

    if (token.startsWith("bad"))
      throw refused(`${service} rejected that token. Check it and try again.`);

    if (token.startsWith("offline")) throw refused(`Could not reach ${service}. Try again.`);

    if (service === "Fizzy" && token.startsWith("none")) {
      throw refused("That token has no Fizzy account to use.");
    }
  };

  const connectGithub = (body: Json | undefined) =>
    integration((held) => {
      checkToken("GitHub", body);

      return {
        settings: {
          ...held,
          profile: { ...held.profile, githubLogin: "riel", githubVerified: true },
          integrations: {
            ...held.integrations,
            github: { state: "connected", name: "riel", workspace: null, appToken: false },
          },
        },
        notice: "GitHub connected as riel.",
      };
    });

  const connectFizzy = (body: Json | undefined) =>
    integration((held) => {
      checkToken("Fizzy", body);

      return {
        settings: {
          ...held,
          integrations: {
            ...held.integrations,
            fizzy: { state: "connected", name: "Riel", workspace: "Smart Data", appToken: false },
          },
        },
        notice: "Fizzy connected as Riel (Smart Data).",
      };
    });

  const disconnect = (service: "github" | "fizzy") =>
    integration((held) => ({
      settings: {
        ...held,
        profile: service === "github" ? { ...held.profile, githubVerified: false } : held.profile,
        integrations: { ...held.integrations, [service]: { state: "missing" } },
      },
      notice: service === "github" ? "GitHub disconnected." : "Fizzy disconnected.",
    }));

  // Dropping Google deletes the meeting cache, so a calendar out of office ends; a manual one
  // stays (`ooo_until_effective`).
  const disconnectGoogle = () =>
    integration((held) => ({
      settings: {
        ...held,
        status: held.status.oooManual ? held.status : { ...held.status, oooUntil: null },
        integrations: {
          ...held.integrations,
          google: {
            ...held.integrations.google,
            connected: false,
            calendar: false,
            drive: false,
            email: null,
          },
        },
      },
      notice: "Google Calendar disconnected.",
    }));

  return {
    appearance: () => {
      const { theme, textSize } = current().settings.appearance;

      return { theme, textSize };
    },
    routes: [
      route("GET", /^\/settings$/, () => ok(page())),
      route("PATCH", /^\/settings\/profile$/, ({ body }) => profile(body)),
      route("PUT", /^\/settings\/avatar$/, ({ body }) => {
        const signedId = stringField(body, "signedId");

        if (signedId === null) throw validation("signedId", "is invalid");

        return avatar(signedId);
      }),
      route("DELETE", /^\/settings\/avatar$/, () => avatar(null)),
      route("PATCH", /^\/settings\/appearance$/, ({ body }) => appearance(body)),
      route("PATCH", /^\/settings\/calls$/, ({ body }) => calls(body)),
      route("PATCH", /^\/settings\/notifications$/, ({ body }) => notifications(body)),
      route("PATCH", /^\/settings\/status$/, ({ body }) => status(body)),
      route("POST", /^\/settings\/dnd_allowances\/(\d+)$/, (request) =>
        allowance(firstId(request), true),
      ),
      route("DELETE", /^\/settings\/dnd_allowances\/(\d+)$/, (request) =>
        allowance(firstId(request), false),
      ),
      route("GET", /^\/settings\/sessions$/, () => ok(sessionList(null))),
      route("POST", /^\/settings\/sessions\/revoke_others$/, () => revokeOthers()),
      route("DELETE", /^\/settings\/sessions\/(\d+)$/, (request) => revoke(firstId(request))),
      route("PUT", /^\/settings\/github_connection$/, ({ body }) => connectGithub(body)),
      route("DELETE", /^\/settings\/github_connection$/, () => disconnect("github")),
      route("PUT", /^\/settings\/fizzy_connection$/, ({ body }) => connectFizzy(body)),
      route("DELETE", /^\/settings\/fizzy_connection$/, () => disconnect("fizzy")),
      route("DELETE", /^\/settings\/google_connection$/, () => disconnectGoogle()),
      route("GET", /^\/settings\/push_subscriptions$/, () => ok(current().push)),
      route("GET", /^\/settings\/push_subscriptions\/key$/, () =>
        ok({ publicKey: MOCK_PUSH_PUBLIC_KEY } satisfies PushPublicKey),
      ),
      route("POST", /^\/settings\/push_subscriptions$/, ({ body }) => createPush(body)),
      route("POST", /^\/settings\/push_subscriptions\/(\d+)\/test$/, (request) => {
        const id = firstId(request);

        if (!current().push.pushSubscriptions.some((subscription) => subscription.id === id)) {
          throw notFound();
        }

        return noContent();
      }),
      route("DELETE", /^\/settings\/push_subscriptions\/(\d+)$/, (request) =>
        removePush(firstId(request)),
      ),
    ],
  };
}
