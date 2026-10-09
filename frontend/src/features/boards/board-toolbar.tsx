import type { BoardOwnerOption } from "../../gen/BoardOwnerOption.ts";
import type { BoardStatusFilter } from "../../gen/BoardStatusFilter.ts";
import type { BoardTagCount } from "../../gen/BoardTagCount.ts";
import { useStore } from "../../store/store.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Menu, MenuGroup, MenuRadioItem, MenuSeparator, SubMenu } from "../../ui/menu.tsx";
import { Tabs } from "../../ui/tabs.tsx";
import {
  activeFilterCount,
  type BoardQuery,
  ownerFilterLabel,
  STATUS_FILTER_LABEL,
} from "./board-format.ts";

const VIEWS = [
  { value: "list", label: "List", icon: "list-checks" },
  { value: "board", label: "Board", icon: "boards" },
] as const;

const STATUS_FILTERS = ["open", "done", "all"] as const satisfies readonly BoardStatusFilter[];

const STATUS_FILTER_HINT = {
  open: "Everything not done yet",
  done: "Only finished posts",
  all: "Every post",
} as const satisfies Record<BoardStatusFilter, string>;

interface BoardToolbarProps {
  readonly query: BoardQuery;
  readonly ownerOptions: readonly BoardOwnerOption[];
  readonly tagCounts: readonly BoardTagCount[];
  readonly onChange: (next: Partial<BoardQuery>) => void;
  readonly onNewPost: () => void;
  /** The board's creator or an administrator: they get the Automations button. */
  readonly canAdminister: boolean;
  readonly automationsOpen: boolean;
  readonly onAutomations: () => void;
}

/** One removable chip per filter that differs from the default. */
function FilterChip({
  label,
  value,
  onClear,
}: {
  readonly label: string;
  readonly value: string;
  readonly onClear: () => void;
}) {
  return (
    <span className="board-chip">
      <span className="board-chip-label">{label}</span>
      <span className="board-chip-value">{value}</span>
      <button
        type="button"
        className="board-chip-clear"
        aria-label={`Clear ${label.toLowerCase()} filter`}
        onClick={onClear}
      >
        <Icon name="x" size={12} />
      </button>
    </span>
  );
}

/**
 * The board's toolbar, under the room header: List or Board on the left, the active filters as
 * chips, then Filter (status in the list, owner, tag, each a submenu of choices), the "+" that
 * starts a new post and, for the board's creator and administrators, Automations, on the right.
 */
export function BoardToolbar({
  query,
  ownerOptions,
  tagCounts,
  onChange,
  onNewPost,
  canAdminister,
  automationsOpen,
  onAutomations,
}: BoardToolbarProps) {
  const users = useStore((state) => state.users);
  const active = activeFilterCount(query);
  const list = query.view === "list";

  return (
    <div className="board-toolbar" role="toolbar" aria-label="Board">
      <Tabs
        items={VIEWS}
        value={query.view}
        label="View"
        onValueChange={(view) => onChange({ view: view === "board" ? "board" : "list" })}
      />
      <div className="board-chips">
        {list && query.status !== "open" ? (
          <FilterChip
            label="Status"
            value={STATUS_FILTER_LABEL[query.status]}
            onClear={() => onChange({ status: "open" })}
          />
        ) : null}
        {query.owner === "anyone" ? null : (
          <FilterChip
            label="Owner"
            value={ownerFilterLabel(query.owner, users)}
            onClear={() => onChange({ owner: "anyone" })}
          />
        )}
        {query.tag === "" ? null : (
          <FilterChip label="Tag" value={query.tag} onClear={() => onChange({ tag: "" })} />
        )}
        {active > 1 ? (
          <Button
            variant="link"
            size="sm"
            className="board-chips-clear"
            onClick={() => onChange({ status: "open", owner: "anyone", tag: "" })}
          >
            Clear
          </Button>
        ) : null}
      </div>
      <div className="board-toolbar-end">
        <Menu
          placement="bottom-end"
          label="Filter posts"
          trigger={(props) => (
            <Button
              {...props}
              variant="ghost"
              size="sm"
              trailingIcon="chevron-down"
              className="board-filter"
              data-active={active > 0 || undefined}
            >
              Filter
              {active > 0 ? <span className="board-filter-count tabular">{active}</span> : null}
            </Button>
          )}
        >
          {list ? (
            <SubMenu label={`Status: ${STATUS_FILTER_LABEL[query.status]}`} icon="circle-dot">
              <MenuGroup label="Status">
                {STATUS_FILTERS.map((status) => (
                  <MenuRadioItem
                    key={status}
                    checked={query.status === status}
                    description={STATUS_FILTER_HINT[status]}
                    onSelect={() => onChange({ status })}
                  >
                    {STATUS_FILTER_LABEL[status]}
                  </MenuRadioItem>
                ))}
              </MenuGroup>
            </SubMenu>
          ) : null}
          <SubMenu label={`Owner: ${ownerFilterLabel(query.owner, users)}`} icon="users">
            <MenuGroup label="Owner">
              <MenuRadioItem
                checked={query.owner === "anyone"}
                onSelect={() => onChange({ owner: "anyone" })}
              >
                Anyone
              </MenuRadioItem>
              <MenuRadioItem
                checked={query.owner === "me"}
                onSelect={() => onChange({ owner: "me" })}
              >
                Me
              </MenuRadioItem>
              <MenuRadioItem
                checked={query.owner === "agents"}
                icon="bot"
                onSelect={() => onChange({ owner: "agents" })}
              >
                Agents
              </MenuRadioItem>
            </MenuGroup>
            {ownerOptions.length > 0 ? <MenuSeparator /> : null}
            {ownerOptions.length > 0 ? (
              <MenuGroup label="Members">
                {ownerOptions.map((option) => {
                  const name = users[option.userId]?.name ?? "Someone";

                  return (
                    <MenuRadioItem
                      key={option.userId}
                      checked={query.owner === String(option.userId)}
                      {...(option.agent ? { icon: "bot" as const } : {})}
                      onSelect={() => onChange({ owner: String(option.userId) })}
                    >
                      {option.agent ? `${name} (agent)` : name}
                    </MenuRadioItem>
                  );
                })}
              </MenuGroup>
            ) : null}
          </SubMenu>
          <SubMenu label={`Tag: ${query.tag === "" ? "Any" : query.tag}`} icon="hash">
            <MenuGroup label="Tag">
              <MenuRadioItem checked={query.tag === ""} onSelect={() => onChange({ tag: "" })}>
                Any tag
              </MenuRadioItem>
              {tagCounts.map((tag) => (
                <MenuRadioItem
                  key={tag.name}
                  checked={query.tag === tag.name}
                  onSelect={() => onChange({ tag: tag.name })}
                >
                  {tag.name} <span className="board-menu-count tabular">({tag.count})</span>
                </MenuRadioItem>
              ))}
            </MenuGroup>
          </SubMenu>
        </Menu>
        <IconButton
          icon="plus"
          label="New post"
          className="board-new"
          tooltipPlacement="bottom"
          onClick={onNewPost}
        />
        {canAdminister ? (
          <IconButton
            icon="settings"
            label="Automations"
            className="board-automations"
            tooltipPlacement="bottom"
            aria-pressed={automationsOpen}
            onClick={onAutomations}
          />
        ) : null}
      </div>
    </div>
  );
}
