import type { RoomCategory } from "../../store/model.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";

interface DeleteCategoryDialogProps {
  readonly category: RoomCategory | null;
  /** How many conversations are in it now. */
  readonly roomCount: number;
  readonly onOpenChange: (open: boolean) => void;
  readonly onConfirm: (category: RoomCategory) => void;
}

/** "Delete Launch?": what happens to its channels, Cancel focused first. */
export function DeleteCategoryDialog({
  category,
  roomCount,
  onOpenChange,
  onConfirm,
}: DeleteCategoryDialogProps) {
  const rooms = roomCount === 1 ? "Its channel goes" : `Its ${roomCount} channels go`;

  return (
    <Dialog
      open={category !== null}
      onOpenChange={onOpenChange}
      role="alertdialog"
      size="sm"
      title={`Delete ${category?.name ?? "category"}?`}
      description={
        roomCount === 0
          ? "The category is empty. Only you see your categories."
          : `${rooms} back to Channels; nothing is deleted but the category. Only you see your categories.`
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
            Delete category
          </Button>
        </>
      }
    />
  );
}
