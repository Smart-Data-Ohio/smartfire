/**
 * Drive search and share for the composer. The mock is always connected: a query filters the
 * seeded files by name, and sharing echoes the members it was asked to grant.
 */
import type { DriveFile } from "../../src/gen/DriveFile.ts";
import type { DriveFileList } from "../../src/gen/DriveFileList.ts";
import type { DriveRecipient } from "../../src/gen/DriveRecipient.ts";
import type { DriveRecipientList } from "../../src/gen/DriveRecipientList.ts";
import type { DriveShare } from "../../src/gen/DriveShare.ts";
import { ok, validation } from "../http.ts";
import { field, isRecord, isString, stringArrayField, stringField } from "../json.ts";
import { USER_IDS } from "../seed.ts";
import { route } from "./context.ts";

const FILES: readonly DriveFile[] = [
  {
    id: "1RoadmapQ4draft",
    name: "Q4 roadmap",
    kind: "document",
    modifiedAt: "2026-09-16T10:30:00.000Z",
    owner: "Maya Okafor",
    url: "https://docs.google.com/document/d/1RoadmapQ4draft/edit",
  },
  {
    id: "2BudgetSheetXx",
    name: "Budget",
    kind: "spreadsheet",
    modifiedAt: "2026-09-15T09:00:00.000Z",
    owner: "Jonah Lindqvist",
    url: "https://docs.google.com/spreadsheets/d/2BudgetSheetXx/edit",
  },
];

const RECIPIENTS: readonly DriveRecipient[] = [
  { id: USER_IDS.maya, name: "Maya Okafor", email: "maya@37signals.com" },
  { id: USER_IDS.jonah, name: "Jonah Lindqvist", email: "jonah@37signals.com" },
];

function listed(query: string): DriveFileList {
  const term = query.trim().toLowerCase();

  return {
    files: FILES.filter((file) => term === "" || (file.name ?? "").toLowerCase().includes(term)),
  };
}

export function createDrive() {
  return {
    routes: [
      route("GET", /^\/drive\/files$/, (request) => ok(listed(request.query.get("q") ?? ""))),
      route("GET", /^\/rooms\/(\d+)\/drive\/recipients$/, () => {
        const body: DriveRecipientList = { recipients: [...RECIPIENTS] };

        return ok(body);
      }),
      route("POST", /^\/rooms\/(\d+)\/drive\/recipients\/validate$/, (request) => {
        const ids = stringArrayField(request.body, "userIds") ?? [];
        const selected = RECIPIENTS.filter((member) => ids.includes(String(member.id)));

        if (selected.length !== ids.length) {
          throw validation("userIds", "invalid_recipients");
        }

        const body: DriveRecipientList = { recipients: selected };

        return ok(body);
      }),
      route("POST", /^\/rooms\/(\d+)\/drive\/shares$/, (request) => {
        const fileId = stringField(request.body, "fileId");
        const approved = field(request.body, "recipients");

        if (fileId === null || fileId === "")
          throw validation("fileId", "includes an invalid file id");

        if (!Array.isArray(approved)) throw validation("recipients", "invalid_recipients");

        const ids: string[] = [];

        for (const entry of approved) {
          if (!isRecord(entry) || !isString(entry.id) || !isString(entry.email)) {
            throw validation("recipients", "invalid_recipients");
          }

          ids.push(entry.id);
        }

        const selected = RECIPIENTS.filter((member) => ids.includes(String(member.id)));

        const body: DriveShare = {
          outcome: "shared",
          fileId,
          blocked: null,
          changedIds: [],
          recipients: [],
          results: selected.map((recipient) => ({ recipient, status: "granted", reason: null })),
        };

        return ok(body);
      }),
    ],
  };
}
