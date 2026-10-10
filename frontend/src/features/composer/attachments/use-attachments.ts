import { useEffect, useRef, useState } from "react";
import {
  browserDeps,
  DEFAULT_UPLOAD_LIMIT_BYTES,
  type UploadSnapshot,
  UploadTask,
  uploadSizeError,
} from "../../../lib/upload/direct-upload.ts";
import type { PendingAttachment } from "../../../store/model.ts";
import { useStore } from "../../../store/store.ts";
import { actions } from "../../../sync/runtime.ts";
import { toast } from "../../../ui/toast-store.ts";

/** One file in the composer's tray. */
export interface TrayFile {
  readonly id: string;
  readonly file: File;
  /** A `blob:` URL for an image's thumbnail, else `null`. */
  readonly previewUrl: string | null;
  readonly snapshot: UploadSnapshot;
}

export interface Attachments {
  readonly files: readonly TrayFile[];
  /** Every file finished uploading (true when there are none). */
  readonly ready: boolean;
  readonly uploading: boolean;
  readonly failed: boolean;
  /** Refuses oversized files, starts valid uploads, and returns how many valid files didn't fit. */
  readonly add: (files: readonly File[]) => number;
  readonly remove: (id: string) => void;
  readonly retry: (id: string) => void;
  /**
   * Removes only the files captured by a send, preserving later additions. Pending rows keep
   * showing the thumbnails, so their URLs are released a little later instead of now.
   */
  readonly clearSent: (sent: readonly TrayFile[]) => void;
}

/** How long a sent image's local preview URL outlives the tray (the pending row uses it). */
const SENT_PREVIEW_TTL_MS = 120_000;

let nextId = 1;

/** Screenshots paste as "image.png": give them a findable name. */
export function pastedName(file: File, now: Date = new Date()): File {
  if (file.name !== "image.png" && file.name !== "") {
    return file;
  }

  const stamp = now.toISOString().slice(0, 19).replace("T", " ").replace(/:/g, ".");
  const extension = file.type.split("/")[1] ?? "png";

  return new File([file], `Pasted image ${stamp}.${extension}`, { type: file.type });
}

/** What a pending message row shows for a tray file. */
export function pendingAttachment(entry: TrayFile): PendingAttachment {
  return {
    filename: entry.file.name,
    contentType: entry.file.type === "" ? "application/octet-stream" : entry.file.type,
    byteSize: entry.file.size,
    previewUrl: entry.previewUrl,
  };
}

/**
 * The composer's attachment tray: each file uploads directly as soon as it's added, and the tray
 * tracks every upload's progress. Unmounting cancels unfinished uploads and frees the previews.
 */
export function useAttachments(max: number): Attachments {
  const limitBytes = useStore(
    (state) => state.boot?.account.uploadLimitBytes ?? DEFAULT_UPLOAD_LIMIT_BYTES,
  );

  const [files, setFiles] = useState<readonly TrayFile[]>([]);
  const tasks = useRef(new Map<string, UploadTask>());

  useEffect(() => {
    const live = tasks.current;

    return () => {
      for (const task of live.values()) {
        task.cancel();
      }

      live.clear();
    };
  }, []);

  // Free every preview still in the tray when the composer goes away.
  const latest = useRef(files);

  useEffect(() => {
    latest.current = files;
  });

  useEffect(
    () => () => {
      for (const entry of latest.current) {
        if (entry.previewUrl !== null) {
          URL.revokeObjectURL(entry.previewUrl);
        }
      }
    },
    [],
  );

  const patch = (id: string, snapshot: UploadSnapshot) => {
    setFiles((current) =>
      current.map((entry) => (entry.id === id ? { ...entry, snapshot } : entry)),
    );
  };

  const add = (incoming: readonly File[]) => {
    const valid = incoming.filter((file) => {
      const error = uploadSizeError(file, limitBytes);

      if (error === null) return true;
      toast({ title: "File too large", description: error, tone: "danger" });

      return false;
    });

    const room = Math.max(0, max - tasks.current.size);
    const taken = valid.slice(0, room);

    const entries = taken.map((file): TrayFile => {
      const id = `upload-${nextId++}`;

      const task = new UploadTask(
        file,
        browserDeps(actions.messages.startUpload, limitBytes),
        (snapshot) => patch(id, snapshot),
      );

      tasks.current.set(id, task);
      void task.start();

      return {
        id,
        file,
        previewUrl: file.type.startsWith("image/") ? URL.createObjectURL(file) : null,
        snapshot: task.snapshot,
      };
    });

    if (entries.length > 0) {
      setFiles((current) => [...current, ...entries]);
    }

    return valid.length - taken.length;
  };

  const remove = (id: string) => {
    tasks.current.get(id)?.cancel();
    tasks.current.delete(id);

    const previewUrl = files.find((entry) => entry.id === id)?.previewUrl ?? null;

    if (previewUrl !== null) {
      URL.revokeObjectURL(previewUrl);
    }

    setFiles((current) => current.filter((entry) => entry.id !== id));
  };

  const retry = (id: string) => {
    void tasks.current.get(id)?.retry();
  };

  const clearSent = (sent: readonly TrayFile[]) => {
    const ids = new Set(sent.map((entry) => entry.id));
    const urls = sent.flatMap((entry) => (entry.previewUrl === null ? [] : [entry.previewUrl]));

    for (const id of ids) {
      tasks.current.delete(id);
    }

    setFiles((current) => current.filter((entry) => !ids.has(entry.id)));
    window.setTimeout(() => {
      for (const url of urls) {
        URL.revokeObjectURL(url);
      }
    }, SENT_PREVIEW_TTL_MS);
  };

  const phases = files.map((entry) => entry.snapshot.phase);

  return {
    files,
    ready: phases.every((phase) => phase === "done"),
    uploading: phases.some(
      (phase) => phase === "hashing" || phase === "starting" || phase === "uploading",
    ),
    failed: phases.some((phase) => phase === "failed"),
    add,
    remove,
    retry,
    clearSent,
  };
}
