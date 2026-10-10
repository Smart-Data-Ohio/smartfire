import type { DrivePick } from "../src/features/composer/drive-picker.ts";
import type { DriveFileList } from "../src/gen/DriveFileList.ts";

/** Local substitute for the external Picker; its catalog includes files search cannot see. */
export async function preparePicker() {
  const response = await fetch("/__mock/drive-picker-files");
  // SAFETY: this mock-only endpoint returns the generated DriveFileList shape.
  const list = (await response.json()) as DriveFileList;
  let dialog: HTMLDialogElement | null = null;
  let cancel = () => {};

  return {
    choose: () =>
      new Promise<DrivePick | null>((resolve) => {
        dialog = document.createElement("dialog");
        dialog.setAttribute("aria-label", "Choose a Drive file");
        cancel = () => {
          dialog?.remove();
          dialog = null;
          resolve(null);
        };

        dialog.addEventListener("cancel", cancel);

        for (const file of list.files) {
          if (file.id === null) continue;

          const pick = {
            id: file.id,
            name: file.name ?? "Google Drive file",
            kind: file.kind,
            url: file.url,
          };

          const button = document.createElement("button");
          button.textContent = pick.name;
          button.type = "button";
          button.addEventListener("click", () => {
            dialog?.remove();
            dialog = null;
            resolve(pick);
          });
          dialog.append(button);
        }

        const close = document.createElement("button");
        close.textContent = "Cancel";
        close.type = "button";
        close.addEventListener("click", cancel);
        dialog.append(close);
        document.body.append(dialog);
        dialog.showModal();
      }),
    dispose: () => cancel(),
  };
}
