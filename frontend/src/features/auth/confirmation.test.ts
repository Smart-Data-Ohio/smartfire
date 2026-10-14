import { describe, expect, it } from "vitest";
import { ActionError } from "../../sync/run.ts";
import { writeMayHaveLanded } from "./confirmation.ts";

describe("whether a failed write may have landed", () => {
  it("counts only a refusal or a stop at the confirmation as nothing saved", () => {
    expect(
      writeMayHaveLanded(new ActionError("Validation", "Invalid", { name: ["is taken"] })),
    ).toBe(false);
    expect(writeMayHaveLanded(new ActionError("ConfirmationCancelled", "Closed"))).toBe(false);
    expect(writeMayHaveLanded(new ActionError("SudoRequired", "Confirm with Google"))).toBe(false);
  });

  it("counts a dropped connection, a timeout or a server error as maybe saved", () => {
    for (const tag of ["NetworkError", "SaveUnconfirmed", "ServerError", "Forbidden"]) {
      expect(writeMayHaveLanded(new ActionError(tag, "Failed"))).toBe(true);
    }

    expect(writeMayHaveLanded(new Error("Failed to fetch"))).toBe(true);
  });
});
