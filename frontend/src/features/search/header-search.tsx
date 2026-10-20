import { useNavigate } from "@tanstack/react-router";
import { useEffect, useEffectEvent, useRef, useState } from "react";
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
  // Whether the person changed the text during this focus (typed, or deleted the filter).
  const [edited, setEdited] = useState(false);
  const [shownFor, setShownFor] = useState(roomId);
  // The field while it has focus.
  const focusedInput = useRef<HTMLInputElement | null>(null);

  // Another room: nothing typed for the last one, its filter least of all, carries over.
  if (shownFor !== roomId) {
    setShownFor(roomId);
    setValue("");
    setPrefill(null);
    setInserted(null);
    setEdited(false);
  }

  const fill = (input: HTMLInputElement, filter: string) => {
    const next = `${filter} `;

    setPrefill(next);
    setInserted(filter);
    setValue(next);
    // A click places the caret where it lands, after this; put it after the filter.
    requestAnimationFrame(() => {
      if (document.activeElement === input && input.value === next) {
        input.setSelectionRange(next.length, next.length);
      }
    });
  };

  // Focused before the room had loaded: fill its filter in once it has, if the field is still
  // empty and untouched.
  const fillLate = useEffectEvent(() => {
    const input = focusedInput.current;

    if (input !== null && token !== null && value === "" && prefill === null && !edited) {
      fill(input, token);
    }
  });

  useEffect(() => {
    if (token !== null) {
      fillLate();
    }
  }, [token]);

  return (
    <div className="header-search">
      <SearchBox
        variant="header"
        value={value}
        onValueChange={(next) => {
          setValue(next);
          setEdited(true);
        }}
        placeholder="Search Messages"
        onFocus={(input) => {
          focusedInput.current = input;
          setEdited(false);
          setPrefill(null);

          if (token !== null && prefillOnFocus(value, token) !== value) {
            fill(input, token);
          }
        }}
        onBlur={() => {
          focusedInput.current = null;
          setValue((current) => settleOnBlur(current, prefill));
          setPrefill(null);
          setEdited(false);
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
