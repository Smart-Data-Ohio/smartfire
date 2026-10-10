import { Schema } from "effect";
import type { ChatSounds as GeneratedChatSounds } from "../../gen/ChatSounds.ts";
import type { DoNotDisturb as GeneratedDoNotDisturb } from "../../gen/DoNotDisturb.ts";
import type { Me as GeneratedMe } from "../../gen/Me.ts";
import type { OutOfOffice as GeneratedOutOfOffice } from "../../gen/OutOfOffice.ts";
import type { Preferences as GeneratedPreferences } from "../../gen/Preferences.ts";
import type { PresenceSetting as GeneratedPresenceSetting } from "../../gen/PresenceSetting.ts";
import type { QuietHours as GeneratedQuietHours } from "../../gen/QuietHours.ts";
import type { TextSize as GeneratedTextSize } from "../../gen/TextSize.ts";
import type { Theme as GeneratedTheme } from "../../gen/Theme.ts";
import type { VoiceMode as GeneratedVoiceMode } from "../../gen/VoiceMode.ts";
import { RoomId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";

export const Theme = Schema.Literals(["system", "light", "dark"]);

export type ThemePin = Assert<Pinned<typeof Theme, GeneratedTheme>>;

export const TextSize = Schema.Literals(["smaller", "small", "default", "large", "larger"]);

export type TextSizePin = Assert<Pinned<typeof TextSize, GeneratedTextSize>>;

export const VoiceMode = Schema.Literals(["voice_activity", "push_to_talk"]);

export type VoiceModePin = Assert<Pinned<typeof VoiceMode, GeneratedVoiceMode>>;

export const PresenceSetting = Schema.Literals(["auto", "dnd", "invisible"]);

export type PresenceSettingPin = Assert<Pinned<typeof PresenceSetting, GeneratedPresenceSetting>>;

export const Preferences = Schema.Struct({
  settingsRevision: Schema.Int,
  theme: Theme,
  textSize: TextSize,
  timeZone: Schema.NullOr(Schema.String),
  timeZoneExplicit: Schema.Boolean,
  tourCompleted: Schema.Boolean,
  voiceMode: VoiceMode,
  pushToTalkKey: Schema.String,
  appearancePreferences: Schema.Json,
});

export type PreferencesPin = Assert<Pinned<typeof Preferences, GeneratedPreferences>>;

export const DoNotDisturb = Schema.Struct({
  enabled: Schema.Boolean,
  until: Schema.NullOr(Timestamp),
});

export type DoNotDisturbPin = Assert<Pinned<typeof DoNotDisturb, GeneratedDoNotDisturb>>;

/** Minutes after midnight in the person's zone; `endMinute` may be before `startMinute`. */
export const QuietHours = Schema.Struct({
  startMinute: Schema.Int,
  endMinute: Schema.Int,
});

export type QuietHoursPin = Assert<Pinned<typeof QuietHours, GeneratedQuietHours>>;

export const ChatSounds = Schema.Struct({
  muted: Schema.Boolean,
  quietHours: Schema.NullOr(QuietHours),
  timeZone: Schema.String,
  quietWindows: Schema.Array(Schema.Tuple([Schema.Int, Schema.Int])),
});

export type ChatSoundsPin = Assert<Pinned<typeof ChatSounds, GeneratedChatSounds>>;

export const OutOfOffice = Schema.Struct({
  until: Timestamp,
  note: Schema.NullOr(Schema.String),
  keepNotifications: Schema.Boolean,
});

export type OutOfOfficePin = Assert<Pinned<typeof OutOfOffice, GeneratedOutOfOffice>>;

/** `GET /api/v1/me` and the boot JSON: the signed-in person and their own settings. */
export const Me = Schema.Struct({
  user: User,
  emailAddress: Schema.NullOr(Schema.String),
  preferences: Preferences,
  presenceSetting: PresenceSetting,
  doNotDisturb: DoNotDisturb,
  quietHours: Schema.NullOr(QuietHours),
  chatSounds: ChatSounds,
  outOfOffice: Schema.NullOr(OutOfOffice),
  /** Where `/app/` opens: the last room visited, else the person's original room. */
  lastRoomId: Schema.NullOr(RoomId),
});

export type Me = typeof Me.Type;

export type MePin = Assert<Pinned<typeof Me, GeneratedMe>>;
