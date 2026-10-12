# Workspace template format

A workspace template is a JSON file containing shared categories and the non-direct rooms the exporting administrator can access. The version 1 format has no database ids or stored template record. Admin settings has a Templates section for export, preview and import.

```json
{
  "format": "smartfire-template",
  "version": 1,
  "categories": [
    {
      "name": "Team",
      "rooms": [
        {
          "kind": "open",
          "name": "General",
          "iconName": null,
          "topic": "Team conversation",
          "announcement": false,
          "voiceBitrate": null,
          "voiceUserLimit": null,
          "board": null
        }
      ]
    }
  ],
  "rooms": []
}
```

Export uses the existing `Room::for_user_without_directs` membership scope plus `Room::of_type(Open)`, the same member/public-room access queries used by the room chooser. Open rooms are included even if the admin has left them. Closed, voice, stage and board rooms are included only when the admin is a member. A restricted room outside that membership scope contributes no name, topic, placement, placeholder or room count. Shared categories remain visible to administrators and are retained when their exported room list is empty. Personal sidebar hiding does not revoke room access.

`categories` follows the shared sidebar's category order. Each category's `rooms` follows its shared room order. Top-level `rooms` contains uncategorized rooms in their shared order. Array order defines placement; position numbers and database ids never enter the file. Export uses the model's ordering, including its name and id tie breakers, and emits fields in a fixed order without timestamps.

| Field | Meaning and source |
| --- | --- |
| `format` | Always `smartfire-template`. |
| `version` | Always `1`. Other versions are refused. |
| `categories[].name` | `workspace_categories.name`. Empty categories are retained. |
| `categories[].rooms`, `rooms` | Accessible, alive non-direct rooms from `WorkspaceCategory::rooms`, filtered through the existing member/public-room scopes before serialization. |
| `kind` | Room type: `open`, `closed`, `voice`, `stage`, or `board`. Open and closed are text rooms. |
| `name` | `rooms.name`, nullable. |
| `iconName` | `rooms.icon_name`, nullable. The named icon must exist in the destination. Custom icon files are excluded. |
| `topic` | `rooms.topic`, nullable. Existing normalization and the 1024-character limit apply. |
| `announcement` | `rooms.announcement`. No follows or publications are copied. |
| `voiceBitrate` | `rooms.voice_bitrate`, nullable. Valid targets are 48000, 64000, 96000, 128000, 256000 and 384000 bits per second. Null uses the destination's default. |
| `voiceUserLimit` | `rooms.voice_user_limit`, nullable. Valid values are 1 through 99. Null means unlimited. |
| `board` | Null for other kinds. Required settings object for boards. |
| `board.tags` | Ordered `board_tags` catalog, each with `name` and nullable `emoji`. At most 20 tags; names have 1 to 20 characters and are unique under the existing Unicode case-folding rule. |
| `board.tagsRequired` | `rooms.tags_required`. Requires at least one catalog tag. |
| `board.defaultTag` | Zero-based array index of `rooms.default_board_tag_id` in the exported catalog, or null. Import maps it to the new tag's id. |

Nullable fields can be omitted on upload and mean null. Unknown fields, unknown kinds, direct rooms, malformed JSON, unsupported formats and versions are refused. Files have a 1 MiB UTF-8 limit. Templates allow at most 100 categories and 1000 rooms. Export refuses a structure that exceeds these limits.

Import appends categories and uncategorized rooms, and preserves each imported group's array order. It creates every room alongside existing rooms. Name comparisons are exact and case sensitive, across all alive non-direct rooms, including private rooms. A conflict gets the first available suffix beginning with ` (2)`. Names reserved by earlier entries in the same file count too. Category names use the same rule within categories; a suffixed category name is shortened to retain its 50-character limit. A conflicting null room name becomes `Unnamed room (2)`.

Preview shows every original and resolved name, counts, settings and validation errors. No entries are skipped. The importer sends the original file text and the resolved structure from preview. The server validates the complete file before opening a write transaction. It checks file size, category and room counts, category and tag name lengths, tag counts and topic lengths first, and stops at the first exceeded limit. Unicode case-folded tag names use a set for duplicate checks. Destination icon validation runs in a read snapshot. Inside the write transaction, the server repeats only destination name allocation to detect concurrent conflicts. A changed conflict result returns 409 and requires a new preview. Validation errors prevent every write; a database failure rolls back categories, rooms, settings, new memberships, audits and pending callbacks together. Room broadcasts run after the transaction succeeds.

The importer becomes each room's creator. Creation uses the same insert, icon validation, membership defaults and callbacks as manual room creation. Template imports write the normal open-room grants to active workspace users inside the import transaction. Manual room creation retains its existing after-commit callback. Closed, voice, stage and board rooms initially contain the importer only; a stage makes the importer its host. No source memberships are copied.

| Endpoint | Request and reply |
| --- | --- |
| `GET /api/v1/admin/templates/export` | Downloads JSON as `smartfire-template-v1.json`, with `Content-Disposition: attachment` and `Cache-Control: no-store`. |
| `POST /api/v1/admin/templates/preview` | JSON `{ "contents": "<file text>" }`. Returns 200 with `template`, `changes`, `errors`, `categoryCount` and `roomCount`. Each change has `entity`, `name` and `createdName`; different names indicate a rename. Parse errors have a null `template`. |
| `POST /api/v1/admin/templates/import` | JSON `{ "contents": "<file text>", "expected": <preview.template> }`. Returns 201 with the accepted preview. Invalid input returns 422; stale preview returns 409. |

All three endpoints use the same administrator gate as the existing SPA admin APIs. Writes use the existing CSRF checks. The request envelope has an 8 MiB limit to accommodate escaped file text and the confirmed structure.

The format excludes roles, memberships, users, personal sidebar categories and favorites, messages, posts, threads, files, branding images, invites, secrets, tokens, email addresses, webhooks, bot credentials, integrations, announcement follows, board automations and user assignments. Those carry content, identities, access grants or integration behavior rather than portable room structure. Workspace appearance, onboarding and AutoMod are outside this format. This checkout has no room-level slow mode, NSFW, admin-only posting or default thread settings to export.
