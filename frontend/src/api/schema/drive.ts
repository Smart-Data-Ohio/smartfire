import { Schema } from "effect";
import type { DriveFile as GeneratedDriveFile } from "../../gen/DriveFile.ts";
import type { DriveFileList as GeneratedDriveFileList } from "../../gen/DriveFileList.ts";
import type { DrivePickerConfig as GeneratedDrivePickerConfig } from "../../gen/DrivePickerConfig.ts";
import type { DriveRecipient as GeneratedDriveRecipient } from "../../gen/DriveRecipient.ts";
import type { DriveRecipientList as GeneratedDriveRecipientList } from "../../gen/DriveRecipientList.ts";
import type { DriveShare as GeneratedDriveShare } from "../../gen/DriveShare.ts";
import type { ShareDriveFile as GeneratedShareDriveFile } from "../../gen/ShareDriveFile.ts";
import type { ValidateDriveRecipients as GeneratedValidateDriveRecipients } from "../../gen/ValidateDriveRecipients.ts";
import { UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";

const nullable = Schema.NullOr(Schema.String);

export const DrivePickerConfig = Schema.Struct({
  clientId: Schema.String,
  apiKey: Schema.String,
  projectNumber: Schema.String,
  accountEmail: nullable,
});

export type DrivePickerConfigPin = Assert<
  Pinned<typeof DrivePickerConfig, GeneratedDrivePickerConfig>
>;

export const DriveFile = Schema.Struct({
  id: nullable,
  name: nullable,
  kind: Schema.String,
  modifiedAt: nullable,
  owner: nullable,
  url: nullable,
});

export type DriveFile = typeof DriveFile.Type;

export type DriveFilePin = Assert<Pinned<typeof DriveFile, GeneratedDriveFile>>;

export const DriveFileList = Schema.Struct({ files: Schema.Array(DriveFile) });

export type DriveFileList = typeof DriveFileList.Type;

export type DriveFileListPin = Assert<Pinned<typeof DriveFileList, GeneratedDriveFileList>>;

export const DriveRecipient = Schema.Struct({
  id: UserId,
  name: Schema.String,
  email: Schema.String,
});

export type DriveRecipient = typeof DriveRecipient.Type;

export type DriveRecipientPin = Assert<Pinned<typeof DriveRecipient, GeneratedDriveRecipient>>;

export const DriveRecipientList = Schema.Struct({ recipients: Schema.Array(DriveRecipient) });

export type DriveRecipientList = typeof DriveRecipientList.Type;

export type DriveRecipientListPin = Assert<
  Pinned<typeof DriveRecipientList, GeneratedDriveRecipientList>
>;

export const ValidateDriveRecipients = Schema.Struct({
  userIds: Schema.Array(Schema.String),
});

export type ValidateDriveRecipients = typeof ValidateDriveRecipients.Type;

export type ValidateDriveRecipientsPin = Assert<
  Pinned<typeof ValidateDriveRecipients, GeneratedValidateDriveRecipients>
>;

export const DriveApprovedRecipient = Schema.Struct({
  id: Schema.String,
  email: Schema.String,
});

export type DriveApprovedRecipient = typeof DriveApprovedRecipient.Type;

export const ShareDriveFile = Schema.Struct({
  fileId: Schema.String,
  recipients: Schema.Array(DriveApprovedRecipient),
  attachedFileIds: Schema.Array(Schema.String),
});

export type ShareDriveFile = typeof ShareDriveFile.Type;

export type ShareDriveFilePin = Assert<Pinned<typeof ShareDriveFile, GeneratedShareDriveFile>>;

export const DriveShareResult = Schema.Struct({
  recipient: DriveRecipient,
  status: Schema.String,
  reason: nullable,
});

export type DriveShareResult = typeof DriveShareResult.Type;

export const DriveShare = Schema.Struct({
  outcome: Schema.String,
  fileId: Schema.String,
  blocked: nullable,
  changedIds: Schema.Array(UserId),
  recipients: Schema.Array(DriveRecipient),
  results: Schema.Array(DriveShareResult),
});

export type DriveShare = typeof DriveShare.Type;

export type DriveSharePin = Assert<Pinned<typeof DriveShare, GeneratedDriveShare>>;
