import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { searchKey } from "../../store/search.ts";
import { useStore } from "../../store/store.ts";
import { useNoteToSelf } from "../directs/note-to-self.ts";
import { prefillOnFocus, roomFilterToken, scopeQuery, settleOnBlur } from "./room-filter.ts";
import { SearchBox } from "./search-box.tsx";

/** The readable filter for the room `roomId` (see `room-filter.ts`), once its kind is known. */
function useRoomFilterToken(roomId: number): string | null {
  const kind = useStore(
    (state) =>
      state.rooms[roomId]?.detail?.room.kind ?? state.sidebar.rows[roomId]?.room.kind ?? null,
  );

  const name = useStore(
    (state) =>
      state.rooms[roomId]?.detail?.displayName ?? state.sidebar.rows[roomId]?.displayName ?? "",
  );

  const noteToSelf = useNoteToSelf(roomId);

  return kind === null ? null : roomFilterToken({ kind, name, noteToSelf });
}

/**
 * The compact search field at the end of a room's header (Discord's place for it): it widens
 * when focused, suggests as you type, and runs the search on its page. Focused empty, it starts
 * with the room's filter (`in:#launch-planning`), plain text the person can delete to search
 * everywhere; submitted untouched, it becomes the room's exact `in_id:`. Hidden on phones, where
 * the sidebar's search button opens the page instead.
 */
export function HeaderSearch({ roomId }: { readonly roomId: number }) {
  const navigate = useNavigate();
  const token = useRoomFilterToken(roomId);
  const [value, setValue] = useState("");
  // The filter this focus filled in, while the field is focused; `null` when it found text.
  const [prefill, setPrefill] = useState<string | null>(null);
  // The filter filled in for this room since the last search, which a submit makes exact.
  const [inserted, setInserted] = useState<string | null>(null);
  const [shownFor, setShownFor] = useState(roomId);

  // Another room: nothing typed for the last one, its filter least of all, carries over.
  if (shownFor !== roomId) {
    setShownFor(roomId);
    setValue("");
    setPrefill(null);
    setInserted(null);
  }

  return (
    <div className="header-search">
      <SearchBox
        variant="header"
        value={value}
        onValueChange={setValue}
        placeholder="Search Messages"
        onFocus={(input) => {
          const next = prefillOnFocus(value, token);

          if (next === value) {
            setPrefill(null);

            return;
          }

          setPrefill(next);
          setInserted(token);
          setValue(next);
          // A click places the caret where it lands, after this; put it after the filter.
          requestAnimationFrame(() => {
            if (document.activeElement === input && input.value === next) {
              input.setSelectionRange(next.length, next.length);
            }
          });
        }}
        onBlur={() => {
          setValue((current) => settleOnBlur(current, prefill));
          setPrefill(null);
        }}
        onSearch={(query) => {
          const key = searchKey(scopeQuery(query, inserted, roomId));

          setValue("");
          setPrefill(null);
          setInserted(null);
          void navigate({ to: "/search", search: key === "" ? {} : { q: key } });
        }}
      />
    </div>
  );
}
