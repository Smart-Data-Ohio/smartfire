import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";

interface DeleteCategoryDialogProps<C extends { readonly name: string }> {
  readonly category: C | null;
  /** A shared workspace category, deleted for everyone (administrators only). */
  readonly shared?: boolean;
  /** How many conversations are in it now. */
  readonly roomCount: number;
  readonly onOpenChange: (open: boolean) => void;
  readonly onConfirm: (category: C) => void;
}

/** What deleting a category does to its channels, and who it changes for. */
function consequence(shared: boolean, roomCount: number): string {
  const rooms = roomCount === 1 ? "Its channel goes" : `Its ${roomCount} channels go`;

  if (shared) {
    return roomCount === 0
      ? "The category is empty. It goes for everyone in the workspace."
      : `${rooms} back to Channels for everyone; nothing is deleted but the category.`;
  }

  return roomCount === 0
    ? "The category is empty. Only you see your categories."
    : `${rooms} back to Channels; nothing is deleted but the category. Only you see your categories.`;
}

/** "Delete Launch?": what happens to its channels, Cancel focused first. */
export function DeleteCategoryDialog<C extends { readonly name: string }>({
  category,
  shared = false,
  roomCount,
  onOpenChange,
  onConfirm,
}: DeleteCategoryDialogProps<C>) {
  return (
    <Dialog
      open={category !== null}
      onOpenChange={onOpenChange}
      role="alertdialog"
      size="sm"
      title={`Delete ${category?.name ?? "category"}${shared ? " for everyone" : ""}?`}
      description={consequence(shared, roomCount)}
      // The category's menu goes with it: land on Channels, where its rooms went.
      returnFocus={() =>
        document.querySelector<HTMLElement>(
          '[data-drop-section="channels"] .sidebar-section-trigger',
        )
      }
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)} data-autofocus>
            Cancel
          </Button>
          <Button
            variant="danger"
            icon="trash"
            onClick={() => {
              if (category !== null) {
                onConfirm(category);
              }

              onOpenChange(false);
            }}
          >
            {shared ? "Delete shared category" : "Delete category"}
          </Button>
        </>
      }
    />
  );
}
