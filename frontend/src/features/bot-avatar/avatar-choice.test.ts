import { afterEach, describe, expect, it, vi } from "vitest";
import { ActionError } from "../../sync/run.ts";
import { avatarFields, avatarUploads, changesAvatar, keptUpload } from "./avatar-choice.ts";

const CAT = { type: "cat" as const, color: "#eaa06f", face: "mouth" as const, accessory: null };

const RENDER = new File(["png"], "bot-avatar-cat.png", { type: "image/png" });

/** Drawing and uploading, stood in for: the render is RENDER, every upload "signed-avatar". */
function stubbed() {
  return {
    render: vi.spyOn(avatarUploads, "render").mockResolvedValue(RENDER),
    upload: vi.spyOn(avatarUploads, "upload").mockResolvedValue("signed-avatar"),
  };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("saving a bot's picture", () => {
  it("uploads a picked avatar's render with its spec", async () => {
    const { render, upload } = stubbed();

    expect(await avatarFields({ kind: "bot", spec: CAT }, null)).toEqual({
      avatar: "signed-avatar",
      botAvatar: CAT,
    });
    expect(render).toHaveBeenCalledWith(CAT);
    expect(upload).toHaveBeenCalledWith(RENDER);
  });

  it("sends nothing for the avatar the bot already has", async () => {
    const { upload } = stubbed();
    const same = { kind: "bot" as const, spec: { ...CAT, color: "#EAA06F" } };

    expect(changesAvatar(same, CAT)).toBe(false);
    expect(await avatarFields(same, CAT)).toEqual({ avatar: null, botAvatar: null });
    expect(upload).not.toHaveBeenCalled();
  });

  it("sends an image alone, and nothing without one or when keeping", async () => {
    const { render, upload } = stubbed();
    const photo = new File(["jpg"], "photo.jpg", { type: "image/jpeg" });

    expect(await avatarFields({ kind: "upload", file: photo }, CAT)).toEqual({
      avatar: "signed-avatar",
      botAvatar: null,
    });
    expect(upload).toHaveBeenCalledWith(photo);
    expect(render).not.toHaveBeenCalled();
    expect(changesAvatar({ kind: "upload", file: null }, CAT)).toBe(false);
    expect(await avatarFields({ kind: "keep" }, CAT)).toEqual({ avatar: null, botAvatar: null });
  });

  it("reuses a form's upload after the confirmation was closed, and lets it go after", async () => {
    const { render, upload } = stubbed();
    const kept = keptUpload();
    const cat = { kind: "bot" as const, spec: CAT };

    upload.mockResolvedValueOnce("first").mockResolvedValueOnce("second");

    expect((await avatarFields(cat, null, kept)).avatar).toBe("first");
    kept.settle(new ActionError("ConfirmationCancelled", "Closed"));
    expect((await avatarFields(cat, null, kept)).avatar).toBe("first");
    expect(render).toHaveBeenCalledTimes(1);

    // Once the write went through, the server holds that upload: the next save sends its own.
    kept.settle(null);
    expect((await avatarFields(cat, null, kept)).avatar).toBe("second");
  });

  it("uploads afresh for another picture, after another failure, or when an upload failed", async () => {
    const { upload } = stubbed();
    const kept = keptUpload();
    const photo = new File(["jpg"], "photo.jpg", { type: "image/jpeg" });

    upload.mockRejectedValueOnce(new Error("offline"));
    await expect(avatarFields({ kind: "upload", file: photo }, null, kept)).rejects.toThrow();
    expect((await avatarFields({ kind: "upload", file: photo }, null, kept)).avatar).toBe(
      "signed-avatar",
    );

    kept.settle(new ActionError("Validation", "Name is taken", { name: ["is taken"] }));
    await avatarFields({ kind: "upload", file: photo }, null, kept);
    await avatarFields({ kind: "bot", spec: CAT }, null, kept);
    expect(upload).toHaveBeenCalledTimes(4);
  });
});
