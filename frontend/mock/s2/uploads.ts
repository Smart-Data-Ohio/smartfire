/**
 * Direct uploads and the files they become: `POST /uploads`, the raw `PUT` to the disk URL,
 * attaching a finished blob to a message, and serving blobs and icon images back as bytes.
 * The `PUT` can be held or slowed down, so upload progress can be screenshotted.
 */
import type { Attachment } from "../../src/gen/Attachment.ts";
import type { AttachmentPreview } from "../../src/gen/AttachmentPreview.ts";
import type { DirectUpload } from "../../src/gen/DirectUpload.ts";
import {
  headerOf,
  type MockBinaryRequest,
  type MockBinaryResponse,
  ok,
  validation,
} from "../http.ts";
import { intField, type Json, stringField } from "../json.ts";
import { animatedIconGif, iconSvg, imageSize } from "./assets.ts";
import { type Route, route, type S2Context } from "./context.ts";
import { base64, md5Base64 } from "./digest.ts";
import { BRAND_ICONS, customIcon } from "./emoji.ts";
import type { BlobRecord } from "./model.ts";

/** A direct upload URL works for this long after `POST /uploads`. */
export const UPLOAD_URL_TTL_MS = 5 * 60_000;

/** The `PUT` target's path prefix. */
export const DISK_PATH = "/rails/active_storage/disk/";

const BLOB_PATH = "/rails/active_storage/blobs/redirect/";

const REPRESENTATION_PATH = "/rails/active_storage/representations/redirect/";

/** The variation key of `resize_to_limit [1200, 800]`, as Active Storage encodes it. */
const THUMBNAIL_VARIATION =
  "eyJfcmFpbHMiOnsiZGF0YSI6eyJmb3JtYXQiOiJ3ZWJwIiwicmVzaXplX3RvX2xpbWl0IjpbMTIwMCw4MDBdfSwicHVyIjoidmFyaWF0aW9uIn19--5b9c6a4e1f0d2c3b8a7e6f5d4c3b2a1908f7e6d5";

/** Whether a path is one `handleBinary` serves. */
export function isBinaryPath(path: string): boolean {
  return (
    path.startsWith("/rails/active_storage/") ||
    path.startsWith("/icons/") ||
    path.startsWith("/assets/icons/brands/")
  );
}

/** `AttachmentView.preview` from the content type. */
export function previewOf(contentType: string): AttachmentPreview {
  if (contentType.startsWith("image/")) return "image";

  return contentType.startsWith("video/") ? "video" : "file";
}

/** The wire attachment for a stored blob. */
export function attachmentOf(blob: BlobRecord): Attachment {
  const filename = encodeURIComponent(blob.filename);
  const url = `${BLOB_PATH}${blob.signedId}/${filename}`;
  const preview = previewOf(blob.contentType);

  return {
    filename: blob.filename,
    contentType: blob.contentType,
    byteSize: blob.byteSize,
    width: blob.width,
    height: blob.height,
    preview,
    url,
    downloadUrl: `${url}?disposition=attachment`,
    thumbnailUrl:
      preview === "image"
        ? `${REPRESENTATION_PATH}${blob.signedId}/${THUMBNAIL_VARIATION}/${filename}`
        : null,
  };
}

/** A signed id the way Rails writes one: base64 JSON, `--`, a hex digest. */
export function signedIdFor(blobId: number, digest: string): string {
  return `${btoa(JSON.stringify({ _rails: { data: blobId, pur: "blob_id" } }))}--${digest}`;
}

/** Stores a finished blob (the seed's files). */
export function storeBlob(
  ctx: Pick<S2Context, "world" | "hex">,
  filename: string,
  contentType: string,
  bytes: Uint8Array,
): BlobRecord {
  const world = ctx.world();
  const id = world.nextBlobId++;
  const size = imageSize(bytes, contentType);

  const blob: BlobRecord = {
    id,
    signedId: signedIdFor(id, ctx.hex(40)),
    token: ctx.hex(48),
    filename,
    contentType,
    byteSize: bytes.length,
    checksum: md5Base64(bytes),
    bytes,
    width: size?.[0] ?? null,
    height: size?.[1] ?? null,
    uploadExpiresAt: 0,
  };

  world.blobs.set(blob.signedId, blob);

  return blob;
}

const EMPTY = new Uint8Array();

const empty = (status: number): MockBinaryResponse => ({
  status,
  contentType: null,
  bytes: EMPTY,
  headers: {},
});

/** The uploads module. */
export interface Uploads {
  readonly routes: readonly Route[];
  /** Serves `PUT /rails/active_storage/disk/:token`, blobs and icons. */
  handleBinary(request: MockBinaryRequest): Promise<MockBinaryResponse>;
  /** The attachment for a finished upload's signed id; 422 otherwise. */
  attachment(signedId: string): Attachment;
  /** While on, upload `PUT`s wait until released. Turning it off releases them. */
  hold(on: boolean): void;
  release(): void;
  /** Delays every upload `PUT`'s response by `ms` (0 turns it off). */
  throttle(ms: number): void;
  /** Held `PUT`s still waiting. */
  pending(): number;
  /** Lets everything go and forgets the settings (on reset). */
  reset(): void;
}

/** An icon uploaded in admin, as `/icons/:name` serves it. */
export interface UploadedIconFile {
  readonly bytes: Uint8Array;
  readonly contentType: string;
  readonly animated: boolean;
}

/** Creates the uploads module. `iconFile` finds an icon uploaded in admin, by name. */
export function createUploads(
  ctx: S2Context,
  limitBytes: () => number,
  iconFile: (name: string) => UploadedIconFile | null = () => null,
): Uploads {
  let holding = false;
  let held: (() => void)[] = [];
  let throttleMs = 0;

  const release = () => {
    const waiting = held;

    held = [];

    for (const run of waiting) run();
  };

  const create = (body: Json | undefined) => {
    const filename = stringField(body, "filename")?.trim() ?? "";
    const byteSize = intField(body, "byteSize");
    const checksum = stringField(body, "checksum") ?? "";
    const contentType = stringField(body, "contentType")?.trim() ?? "";

    if (filename === "") throw validation("filename", "Filename can't be blank");

    if (byteSize === null || byteSize < 0) {
      throw validation("byteSize", "Byte size must be greater than or equal to 0");
    }

    if (checksum === "") throw validation("checksum", "Checksum can't be blank");
    const limit = limitBytes();

    if (byteSize > limit) {
      const mb = 1024 * 1024;
      const size = limit % mb === 0 ? `${limit / mb} MB` : `${limit} bytes`;
      throw validation("byteSize", `File exceeds the ${size} upload limit.`);
    }

    const world = ctx.world();
    const id = world.nextBlobId++;

    const blob: BlobRecord = {
      id,
      signedId: signedIdFor(id, ctx.hex(40)),
      token: ctx.hex(48),
      filename,
      contentType: contentType === "" ? "application/octet-stream" : contentType,
      byteSize,
      checksum,
      bytes: null,
      width: null,
      height: null,
      uploadExpiresAt: ctx.now() + UPLOAD_URL_TTL_MS,
    };

    world.blobs.set(blob.signedId, blob);

    const upload: DirectUpload = {
      signedId: blob.signedId,
      uploadUrl: `${DISK_PATH}${blob.token}`,
    };

    return ok(upload);
  };

  const blobByToken = (token: string): BlobRecord | null => {
    for (const blob of ctx.world().blobs.values()) {
      if (blob.token === token) return blob;
    }

    return null;
  };

  /** Active Storage's disk `update`: the bytes must match what was declared. */
  const put = (token: string, request: MockBinaryRequest): MockBinaryResponse => {
    const blob = blobByToken(token);

    if (blob === null || ctx.now() > blob.uploadExpiresAt) return empty(404);

    const bytes = request.bytes ?? EMPTY;
    const sentType = (headerOf(request.headers, "content-type") ?? "").split(";")[0]?.trim() ?? "";

    const matches =
      bytes.length === blob.byteSize &&
      sentType === blob.contentType &&
      md5Base64(bytes) === blob.checksum;

    if (!matches) return empty(422);

    const size = imageSize(bytes, blob.contentType);

    blob.bytes = bytes;
    blob.width = size?.[0] ?? null;
    blob.height = size?.[1] ?? null;

    return empty(204);
  };

  const delayed = (run: () => MockBinaryResponse): Promise<MockBinaryResponse> => {
    const answer = () =>
      new Promise<MockBinaryResponse>((resolve) => {
        if (throttleMs <= 0) {
          resolve(run());

          return;
        }

        ctx.scheduler.schedule(throttleMs, () => resolve(run()));
      });

    if (!holding) return answer();

    return new Promise((resolve) => {
      held.push(() => {
        answer().then(resolve, () => resolve(empty(500)));
      });
    });
  };

  const serveBlob = (signedId: string, filename: string, download: boolean): MockBinaryResponse => {
    const blob = ctx.world().blobs.get(signedId);

    if (blob === undefined || blob.bytes === null) return empty(404);

    const disposition = download ? "attachment" : "inline";
    const name = decodeURIComponent(filename);

    return {
      status: 200,
      contentType: blob.contentType,
      bytes: blob.bytes,
      headers: {
        "Content-Disposition": `${disposition}; filename="${name.replace(/"/g, "")}"; filename*=UTF-8''${encodeURIComponent(name)}`,
        "Cache-Control": "max-age=300, private",
        ETag: `"${base64(new TextEncoder().encode(blob.checksum)).slice(0, 16)}"`,
      },
    };
  };

  /**
   * A brand or workspace icon's image. An animated workspace icon (the GIF fixture, or an upload
   * that moves) answers `?still=1` with a first frame, drawn here as the icon's badge; any other
   * icon answers it with its original, as the server does.
   */
  const serveIcon = (path: string, still: boolean): MockBinaryResponse => {
    const brand = /^\/assets\/icons\/brands\/([a-z0-9_]+)\.svg$/.exec(path)?.[1];
    const custom = /^\/icons\/([a-z0-9_]+)$/.exec(path)?.[1];
    const uploaded = custom === undefined ? null : iconFile(custom);
    const fixture = custom === undefined ? undefined : customIcon(custom);

    const icon = (contentType: string, bytes: Uint8Array): MockBinaryResponse => ({
      status: 200,
      contentType,
      bytes,
      headers: { "Cache-Control": "max-age=3600" },
    });

    if (uploaded !== null && !(still && uploaded.animated)) {
      return icon(uploaded.contentType, uploaded.bytes);
    }

    if (fixture?.animated === true && !still) return icon("image/gif", animatedIconGif());

    const known =
      (brand !== undefined && BRAND_ICONS.some((each) => each.name === brand)) ||
      fixture !== undefined ||
      uploaded !== null;

    return known ? icon("image/svg+xml", iconSvg(brand ?? custom ?? "")) : empty(404);
  };

  return {
    routes: [route("POST", /^\/uploads$/, (request) => create(request.body))],
    async handleBinary(request) {
      const url = new URL(request.path, "http://mock.invalid");
      const path = url.pathname;
      const method = request.method.toUpperCase();

      if (path.startsWith(DISK_PATH)) {
        if (method !== "PUT") return empty(405);

        return delayed(() => put(path.slice(DISK_PATH.length), request));
      }

      if (method !== "GET" && method !== "HEAD") return empty(405);

      const blob = /^\/rails\/active_storage\/blobs\/(?:redirect|proxy)\/([^/]+)\/([^/]+)$/.exec(
        path,
      );

      if (blob !== null) {
        return serveBlob(
          blob[1] ?? "",
          blob[2] ?? "",
          url.searchParams.get("disposition") === "attachment",
        );
      }

      const representation =
        /^\/rails\/active_storage\/representations\/(?:redirect|proxy)\/([^/]+)\/[^/]+\/([^/]+)$/.exec(
          path,
        );

      if (representation !== null)
        return serveBlob(representation[1] ?? "", representation[2] ?? "", false);

      return path.startsWith("/icons/") || path.startsWith("/assets/icons/")
        ? serveIcon(path, url.searchParams.get("still") === "1")
        : empty(404);
    },
    attachment(signedId) {
      const blob = ctx.world().blobs.get(signedId);

      if (blob === undefined) throw validation("attachment", "Attachment is invalid");

      if (blob.bytes === null)
        throw validation("attachment", "Attachment hasn't finished uploading");

      return attachmentOf(blob);
    },
    hold(on) {
      holding = on;

      if (!on) release();
    },
    release,
    throttle(ms) {
      throttleMs = Math.max(0, ms);
    },
    pending: () => held.length,
    reset() {
      holding = false;
      throttleMs = 0;
      release();
    },
  };
}
