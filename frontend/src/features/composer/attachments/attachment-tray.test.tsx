import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, expect, it } from "vitest";
import { AttachmentTray } from "./attachment-tray.tsx";
import { fileOptions, type TrayFile } from "./use-attachments.ts";

const ready = (name: string): TrayFile => ({
  id: name,
  file: new File(["hello"], name, { type: "image/png" }),
  spoiler: false,
  previewUrl: `blob:${name}`,
  snapshot: { phase: "done", loaded: 5, total: 5, signedId: `signed-${name}`, error: null },
});

function Tray() {
  const [files, setFiles] = useState([ready("one.png"), ready("two.png")]);

  return (
    <>
      <AttachmentTray
        files={files}
        onRemove={() => {}}
        onRetry={() => {}}
        onToggleSpoiler={(id) =>
          setFiles((current) =>
            current.map((entry) =>
              entry.id === id ? { ...entry, spoiler: !entry.spoiler } : entry,
            ),
          )
        }
      />
      <output>{JSON.stringify(fileOptions(files))}</output>
    </>
  );
}

afterEach(cleanup);

it("toggles one file with keyboard and sends mixed flags in tray order", async () => {
  const { container } = render(<Tray />);
  const first = screen.getByRole("button", { name: "Mark one.png as spoiler" });
  const second = screen.getByRole("button", { name: "Mark two.png as spoiler" });
  first.focus();
  await userEvent.setup().keyboard("{Enter}");
  expect(first.getAttribute("aria-pressed")).toBe("true");
  expect(second.getAttribute("aria-pressed")).toBe("false");
  expect(container.querySelector("output")?.textContent).toContain(
    '"attachmentSpoilers":[true,false]',
  );
  await userEvent.setup().click(first);
  expect(first.getAttribute("aria-pressed")).toBe("false");
});
