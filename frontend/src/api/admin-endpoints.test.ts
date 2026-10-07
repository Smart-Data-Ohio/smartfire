import { describe, expect, it } from "vitest";
import { auditLogQuery } from "./admin-endpoints.ts";

const NONE = { actor: null, action: null, targetType: null, from: null, to: null };

describe("the audit log query", () => {
  it("sends only what's set, with the API's names", () => {
    expect(auditLogQuery({ ...NONE, action: "user.role.change", targetType: "User" }, "2")).toEqual(
      { action: "user.role.change", targetType: "User", page: "2" },
    );
    expect(auditLogQuery({ ...NONE, actor: "  " }, null)).toEqual({});
  });
});
