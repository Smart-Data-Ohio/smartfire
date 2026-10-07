import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { searchKey } from "../../store/search.ts";
import { SearchBox } from "./search-box.tsx";

/**
 * The compact search field at the end of a room's header (Discord's place for it): it widens
 * when focused, suggests as you type, and runs the search on its page. Hidden on phones, where
 * the sidebar's search button opens the page instead.
 */
export function HeaderSearch() {
  const navigate = useNavigate();
  const [value, setValue] = useState("");

  return (
    <div className="header-search">
      <SearchBox
        variant="header"
        value={value}
        onValueChange={setValue}
        placeholder="Search"
        onSearch={(query) => {
          const key = searchKey(query);

          setValue("");
          void navigate({ to: "/search", search: key === "" ? {} : { q: key } });
        }}
      />
    </div>
  );
}
