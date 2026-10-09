import { describe, expect, it } from "vitest";
import { imageProblem, PROFILE_IMAGE_MAX_BYTES, uploadPercent } from "./profile-format.ts";

describe("imageProblem", () => {
  it("takes PNG, JPEG, GIF and WebP up to 10 MB", () => {
    for (const type of ["image/png", "image/jpeg", "image/gif", "image/webp"]) {
      expect(imageProblem({ type, size: PROFILE_IMAGE_MAX_BYTES })).toBeNull();
    }
  });

  it("refuses other files and larger images", () => {
    expect(imageProblem({ type: "image/svg+xml", size: 10 })).toBe(
      "Choose a PNG, JPEG, GIF or WebP image.",
    );

    expect(imageProblem({ type: "image/png", size: PROFILE_IMAGE_MAX_BYTES + 1 })).toBe(
      "Choose an image of 10 MB or less.",
    );
  });
});

describe("uploadPercent", () => {
  it("rounds, caps at 100 and reads an unknown total as nothing yet", () => {
    expect(uploadPercent(1, 3)).toBe(33);

    expect(uploadPercent(5, 4)).toBe(100);

    expect(uploadPercent(5, 0)).toBe(0);
  });
});
