import { useNavigate, useSearch } from "@tanstack/react-router";
import type { ActivityState } from "../../gen/ActivityState.ts";
import type { ActivityTab } from "../../gen/ActivityTab.ts";
import { ActivityPage } from "./activity-page.tsx";
import { parseActivitySearch } from "./activity-search.ts";

/** `/app/activity`: the inbox, its tab and state filter kept in the URL so Back and links work. */
export function ActivityRoute() {
  const search = useSearch({ from: "/shell/activity" });
  const navigate = useNavigate();

  const onFilterChange = (tab: ActivityTab, status: ActivityState) => {
    void navigate({
      to: "/activity",
      search: parseActivitySearch({ tab, status }),
      replace: true,
    });
  };

  return (
    <ActivityPage
      tab={search.tab ?? "all"}
      status={search.status ?? "unread"}
      onFilterChange={onFilterChange}
    />
  );
}
