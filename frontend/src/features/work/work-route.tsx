import { useNavigate, useSearch } from "@tanstack/react-router";
import type { WorkFilter } from "../../gen/WorkFilter.ts";
import { WorkPage } from "./work-page.tsx";
import { parseWorkSearch } from "./work-search.ts";

/** `/app/work`: tracked work from every room the viewer is in, its filter kept in the URL. */
export function WorkRoute() {
  const search = useSearch({ from: "/shell/work" });
  const navigate = useNavigate();

  const onFilterChange = (state: WorkFilter) => {
    void navigate({ to: "/work", search: parseWorkSearch({ state }), replace: true });
  };

  return <WorkPage filter={search.state ?? "open"} onFilterChange={onFilterChange} />;
}
