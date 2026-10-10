import { Schema } from "effect";
import { DriveFileList } from "../src/api/schema/drive.ts";
import type { DrivePick } from "../src/features/composer/drive-picker.ts";
import type { DrivePickerConfig } from "../src/gen/DrivePickerConfig.ts";

const PickerFiles = Schema.Struct({ ...DriveFileList.fields, accountEmail: Schema.String });

/** Local substitute for the external Picker; its catalog includes files search cannot see. */
export async function preparePicker(config: DrivePickerConfig) {
  const response = await fetch("/__mock/drive-picker-files");
  const list = Schema.decodeUnknownSync(PickerFiles)(await response.json());
  let dialog: HTMLDialogElement | null = null;
  let cancel = () => {};

  return {
    choose: () =>
      new Promise<DrivePick | null>((resolve, reject) => {
        if (
          config.accountEmail &&
          list.accountEmail.toLowerCase() !== config.accountEmail.trim().toLowerCase()
        ) {
          reject(
            new Error(
              `Pick files from ${config.accountEmail}, the Google account connected to Smartfire`,
            ),
          );

          return;
        }

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
