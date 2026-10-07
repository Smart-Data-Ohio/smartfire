/** The people and rooms search suggests, from the switcher's data. */
import { useStore } from "../../store/store.ts";
import { localItems, mergeItems, remoteItems, type SwitcherItem } from "../switcher/ranking.ts";
import { useCatalogue } from "../switcher/use-catalogue.ts";

/** The people and rooms the typeahead suggests: the sidebar's, then the server's catalogue. */
export function useSuggestible(active: boolean): readonly SwitcherItem[] {
  const sidebar = useStore((state) => state.sidebar);
  const users = useStore((state) => state.users);
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  const { catalogue } = useCatalogue(active);

  return mergeItems(
    localItems(sidebar, viewerId),
    catalogue === null ? [] : remoteItems(catalogue),
    (userId) => users[userId]?.name,
  ).filter((item) => item.userId !== viewerId);
}
