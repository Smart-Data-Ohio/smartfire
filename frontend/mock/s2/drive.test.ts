import { describe, expect, it } from "vitest";
import type { DriveFileList } from "../../src/gen/DriveFileList.ts";
import type { DrivePickerConfig } from "../../src/gen/DrivePickerConfig.ts";
import { get, harness } from "./testing.ts";

describe("Drive Picker mock", () => {
  it("exposes Picker configuration and files outside the stored search grant", async () => {
    const { server } = harness();
    expect(await get<DrivePickerConfig>(server, "/api/v1/drive/picker")).toEqual({
      clientId: "mock-client",
      apiKey: "mock-key",
      projectNumber: "123456",
    });
    const search = await get<DriveFileList>(server, "/api/v1/drive/files?q=private");
    const picker = await get<DriveFileList>(server, "/__mock/drive-picker-files");
    expect(search.files).toEqual([]);
    expect(picker.files.map((file) => file.id)).toContain("3ExistingDriveFile");
  });
});
