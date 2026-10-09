import { describe, expect, it, vi } from "vitest";
import { registerWorker } from "./register.ts";

describe("SPA worker registration", () => {
  it("updates the installed root worker at /service-worker.js with scope /", async () => {
    const update = vi.fn(async () => undefined);
    const register = vi.fn(async () => ({ update }));
    await registerWorker("/service-worker.js", { register });

    expect(register).toHaveBeenCalledExactlyOnceWith("/service-worker.js", {
      scope: "/",
      updateViaCache: "none",
    });
    expect(update).toHaveBeenCalledOnce();
  });

  it("does not register when boot disables enrollment or the browser lacks workers", async () => {
    const register = vi.fn();

    await registerWorker(null, { register });
    await registerWorker("/service-worker.js", null);

    expect(register).not.toHaveBeenCalled();
  });
});
