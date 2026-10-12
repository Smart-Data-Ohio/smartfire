import { useRef, useState } from "react";
import type { TemplatePreview } from "../../gen/TemplatePreview.ts";
import type { TemplateRoom } from "../../gen/TemplateRoom.ts";
import { templates } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { SettingsGroup, SettingsPage } from "../settings/settings-parts.tsx";
import { AdministratorsOnly, useAdmin } from "./admin-parts.tsx";
import "./templates.css";

const FILE_LIMIT = 1024 * 1024;

type Flow =
  | { status: "idle" }
  | { status: "previewing"; filename: string }
  | { status: "preview"; filename: string; contents: string; preview: TemplatePreview }
  | { status: "importing"; filename: string; preview: TemplatePreview }
  | { status: "success"; message: string }
  | { status: "error"; message: string };

function RoomSettings({ room }: { readonly room: TemplateRoom }) {
  return (
    <li>
      <details>
        <summary>
          {room.name ?? "Unnamed room"} ({room.kind})
        </summary>
        <dl className="template-settings">
          <dt>Topic</dt>
          <dd>{room.topic ?? "None"}</dd>
          <dt>Icon</dt>
          <dd>{room.iconName ?? "Default"}</dd>
          <dt>Announcements</dt>
          <dd>{room.announcement ? "On" : "Off"}</dd>
          <dt>Voice bitrate</dt>
          <dd>{room.voiceBitrate === null ? "Default" : `${room.voiceBitrate} bps`}</dd>
          <dt>Voice user limit</dt>
          <dd>{room.voiceUserLimit ?? "Unlimited"}</dd>
          {room.board === null ? null : (
            <>
              <dt>Tags</dt>
              <dd>
                {room.board.tags.map((tag) => `${tag.emoji ?? ""} ${tag.name}`.trim()).join(", ") ||
                  "None"}
              </dd>
              <dt>Tags required</dt>
              <dd>{room.board.tagsRequired ? "Yes" : "No"}</dd>
              <dt>Default tag</dt>
              <dd>
                {room.board.defaultTag === null
                  ? "None"
                  : room.board.tags[room.board.defaultTag]?.name}
              </dd>
            </>
          )}
        </dl>
      </details>
    </li>
  );
}

function Preview({ preview }: { readonly preview: TemplatePreview }) {
  return (
    <section className="template-preview" aria-label="Template preview">
      <p>
        {preview.categoryCount} categories and {preview.roomCount} rooms will be created. Nothing
        will be skipped.
      </p>
      <ul aria-label="Planned changes">
        {preview.changes.map((change) => (
          <li key={`${change.entity}-${JSON.stringify(change.createdName)}`}>
            {change.entity === "category" ? "Category" : "Room"} {change.name ?? "Unnamed room"}
            {change.name === change.createdName
              ? " will be created"
              : ` will be created as ${change.createdName ?? "Unnamed room"}`}
          </li>
        ))}
      </ul>
      {preview.errors.length === 0 ? null : (
        <div role="alert">
          <p>Fix these errors before importing:</p>
          <ul>
            {[...new Set(preview.errors)].map((error) => (
              <li key={error}>{error}</li>
            ))}
          </ul>
        </div>
      )}
      {preview.template === null ? null : (
        <div>
          {preview.template.categories.map((category) => (
            <div key={category.name}>
              <h3>{category.name}</h3>
              <ul>
                {category.rooms.map((room) => (
                  <RoomSettings key={JSON.stringify(room.name)} room={room} />
                ))}
              </ul>
            </div>
          ))}
          {preview.template.rooms.length === 0 ? null : (
            <>
              <h3>Uncategorized</h3>
              <ul>
                {preview.template.rooms.map((room) => (
                  <RoomSettings key={JSON.stringify(room.name)} room={room} />
                ))}
              </ul>
            </>
          )}
        </div>
      )}
    </section>
  );
}

function readFile(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();

    reader.onload = () =>
      reader.result !== null && !(reader.result instanceof ArrayBuffer)
        ? resolve(reader.result)
        : reject(new Error("Couldn't read the template file."));
    reader.onerror = () => reject(new Error("Couldn't read the template file."));
    reader.readAsText(file);
  });
}

function TemplatesForm() {
  const [flow, setFlow] = useState<Flow>({ status: "idle" });
  const [exportState, setExportState] = useState<"idle" | "saving" | "success">("idle");
  const [exportError, setExportError] = useState<string | null>(null);
  const request = useRef(0);
  const previewHeading = useRef<HTMLHeadingElement>(null);
  const busy = flow.status === "previewing" || flow.status === "importing";

  const download = async () => {
    setExportState("saving");
    setExportError(null);

    try {
      const template = await templates.export();

      const url = URL.createObjectURL(
        new Blob([JSON.stringify(template, null, 2)], { type: "application/json" }),
      );

      const link = document.createElement("a");

      link.href = url;
      link.download = "smartfire-template-v1.json";
      link.click();
      const revoke = URL.revokeObjectURL;

      setTimeout(() => revoke(url), 1000);
      setExportState("success");
    } catch (error) {
      setExportState("idle");
      setExportError(error instanceof Error ? error.message : "Couldn't export the template.");
    }
  };

  const select = async (file: File | undefined) => {
    const current = ++request.current;
    setFlow({ status: "idle" });

    if (file === undefined) return;

    if (file.size > FILE_LIMIT) {
      setFlow({ status: "error", message: "Template files must be at most 1 MiB." });

      return;
    }

    setFlow({ status: "previewing", filename: file.name });

    try {
      const contents = await readFile(file);
      const preview = await templates.preview(contents);

      if (current !== request.current) return;
      setFlow({ status: "preview", filename: file.name, contents, preview });
      requestAnimationFrame(() => previewHeading.current?.focus());
    } catch (error) {
      if (current === request.current)
        setFlow({
          status: "error",
          message: error instanceof Error ? error.message : "Couldn't preview the template.",
        });
    }
  };

  const confirm = async () => {
    if (
      flow.status !== "preview" ||
      flow.preview.template === null ||
      flow.preview.errors.length > 0
    )
      return;
    const { filename, contents, preview } = flow;
    const expected = flow.preview.template;
    setFlow({ status: "importing", filename, preview });

    try {
      const result = await templates.import(contents, expected);
      setFlow({
        status: "success",
        message: `Created ${result.categoryCount} categories and ${result.roomCount} rooms.`,
      });
    } catch (error) {
      setFlow({
        status: "error",
        message:
          error instanceof Error
            ? error.message
            : "Couldn't import the template. Choose the file to preview again.",
      });
    }
  };

  return (
    <SettingsPage
      title="Templates"
      description="Save your workspace structure or create rooms from another Smartfire workspace."
    >
      <SettingsGroup
        title="Export structure"
        description="Includes shared categories, rooms and their settings. People, memberships, messages, files and integrations are excluded."
      >
        <Button
          disabled={exportState === "saving"}
          loading={exportState === "saving"}
          onClick={() => {
            void download();
          }}
        >
          Export template
        </Button>
        {exportState === "success" ? <p role="status">Template downloaded.</p> : null}
        {exportError === null ? null : <p role="alert">{exportError}</p>}
      </SettingsGroup>
      <SettingsGroup
        title="Import structure"
        description="Choose a Smartfire JSON template up to 1 MiB. Existing rooms stay in place; conflicting names get a numbered suffix. Open rooms use normal workspace membership rules. Other rooms initially include only you."
      >
        <label htmlFor="template-file">Template file</label>
        <input
          className="input template-file"
          id="template-file"
          type="file"
          accept="application/json,.json"
          disabled={busy}
          onChange={(event) => {
            void select(event.target.files?.[0]);
            event.target.value = "";
          }}
        />
        {flow.status === "previewing" ? <p role="status">Previewing {flow.filename}…</p> : null}
        {flow.status === "preview" || flow.status === "importing" ? (
          <>
            <h2 ref={previewHeading} tabIndex={-1}>
              Preview: {flow.filename}
            </h2>
            <Preview preview={flow.preview} />
            <div className="settings-actions">
              <Button
                onClick={() => {
                  request.current++;
                  setFlow({ status: "idle" });
                }}
                disabled={busy}
              >
                Cancel
              </Button>
              <Button
                variant="primary"
                loading={busy}
                disabled={
                  busy ||
                  flow.preview.template === null ||
                  flow.preview.errors.length > 0 ||
                  flow.preview.roomCount + flow.preview.categoryCount === 0
                }
                onClick={() => {
                  void confirm();
                }}
              >
                Confirm import
              </Button>
            </div>
          </>
        ) : null}
        {flow.status === "success" ? <p role="status">{flow.message}</p> : null}
        {flow.status === "error" ? <p role="alert">{flow.message}</p> : null}
      </SettingsGroup>
    </SettingsPage>
  );
}

export function TemplatesSection() {
  const { workspace } = useAdmin();

  return workspace.canAdminister ? <TemplatesForm /> : <AdministratorsOnly />;
}
