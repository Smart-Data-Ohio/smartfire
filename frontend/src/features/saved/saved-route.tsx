import { useNavigate, useSearch } from "@tanstack/react-router";
import type { SavedFilter } from "../../gen/SavedFilter.ts";
import { SavedPage } from "./saved-page.tsx";
import { parseSavedSearch } from "./saved-search.ts";

/** `/app/saved`: the Saved page, its filter kept in the URL. */
export function SavedRoute() {
  const search = useSearch({ from: "/shell/saved" });
  const navigate = useNavigate();

  const onFilterChange = (status: SavedFilter) => {
    void navigate({ to: "/saved", search: parseSavedSearch({ status }), replace: true });
  };

  return <SavedPage filter={search.status ?? "in_progress"} onFilterChange={onFilterChange} />;
}
