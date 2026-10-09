/** Drive search and the room share grant (`/api/v1/drive`, `/api/v1/rooms/:id/drive`). */
import { Effect } from "effect";
import type { DriveFile } from "../gen/DriveFile.ts";
import type { DriveFileList } from "../gen/DriveFileList.ts";
import type { DriveRecipientList } from "../gen/DriveRecipientList.ts";
import type { DriveShare } from "../gen/DriveShare.ts";
import { call, get } from "./call.ts";
import {
  DriveFileList as DriveFileListSchema,
  DriveFile as DriveFileSchema,
  DriveRecipientList as DriveRecipientListSchema,
  DriveShare as DriveShareSchema,
} from "./schema/drive.ts";
import { wire } from "./wire.ts";

/** `GET /drive/files?q=`: the viewer's files, recent first, filtered by name. */
export const searchDriveFiles = Effect.fn("api.searchDriveFiles")(function* (query: string) {
  return yield* call(get("/drive/files", { q: query }), wire<DriveFileList>(DriveFileListSchema));
});

/** `GET /drive/files/:id`: one file's metadata, cached per viewer. */
export const readDriveFile = Effect.fn("api.readDriveFile")(function* (fileId: string) {
  return yield* call(get(`/drive/files/${fileId}`), wire<DriveFile>(DriveFileSchema));
});

/** `GET /rooms/:id/drive/recipients`: members the viewer may grant, no Drive account required. */
export const driveRecipients = Effect.fn("api.driveRecipients")(function* (roomId: number) {
  return yield* call(
    get(`/rooms/${roomId}/drive/recipients`),
    wire<DriveRecipientList>(DriveRecipientListSchema),
  );
});

/**
 * `POST /rooms/:id/drive/shares`: reader access for the chosen members, with email off.
 * A viewer with no Drive grant gets 404, the same empty miss as search.
 */
export const shareDriveFile = Effect.fn("api.shareDriveFile")(function* (
  roomId: number,
  fileId: string,
  recipients: readonly { readonly id: string; readonly email: string }[],
  attachedFileIds: readonly string[],
) {
  return yield* call(
    {
      method: "POST",
      path: `/rooms/${roomId}/drive/shares`,
      body: {
        fileId,
        recipients: recipients.map((member) => ({ id: member.id, email: member.email })),
        attachedFileIds: [...attachedFileIds],
      },
    },
    wire<DriveShare>(DriveShareSchema),
  );
});
