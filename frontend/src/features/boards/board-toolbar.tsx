import type { ReactNode } from "react";
import type { BoardOwnerOption } from "../../gen/BoardOwnerOption.ts";
import type { BoardStatusFilter } from "../../gen/BoardStatusFilter.ts";
import type { BoardTag } from "../../gen/BoardTag.ts";
import type { BoardTagCount } from "../../gen/BoardTagCount.ts";
import { useStore } from "../../store/store.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import {
  Menu,
  MenuGroup,
  MenuItem,
  MenuRadioItem,
  MenuSeparator,
  SubMenu,
} from "../../ui/menu.tsx";
import { Tabs } from "../../ui/tabs.tsx";
import {
  activeFilterCount,
  type BoardQuery,
  ownerFilterLabel,
  STATUS_FILTER_LABEL,
} from "./board-format.ts";
import { TagEmoji } from "./board-parts.tsx";
import { catalogTag, postsUsing, sameTagName } from "./board-tags.ts";

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
  /** The board's curated tags, which lead the tag filter with their emoji. */
  readonly catalog: readonly BoardTag[];
  readonly onChange: (next: Partial<BoardQuery>) => void;
  readonly onNewPost: () => void;
  /** The board's creator or an administrator: they get the Automations button. */
  readonly canAdminister: boolean;
  readonly automationsOpen: boolean;
  readonly onAutomations: () => void;
  /** Opens the board's tag editor (its settings' Tags tab); only for those who may edit it. */
  readonly onManageTags: () => void;
}

/** One removable chip per filter that differs from the default. */
function FilterChip({
  label,
  value,
  onClear,
}: {
  readonly label: string;
  readonly value: ReactNode;
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

/** The tag filter's name for a tag: a curated tag's emoji and label, else the tag as typed. */
function TagValue({
  catalog,
  tag,
}: {
  readonly catalog: readonly BoardTag[];
  readonly tag: string;
}) {
  const curated = catalogTag(catalog, tag);

  return curated === undefined ? (
    tag
  ) : (
    <>
      <TagEmoji emoji={curated.emoji} />
      {curated.name}
    </>
  );
}

/**
 * The board's curated tags as a row of filter chips under the toolbar (Discord's forum tag bar):
 * one press filters to a tag, a second press on it clears the filter. Free-text tags stay in the
 * Filter menu. Scrolls sideways on a phone rather than wrapping.
 */
export function BoardTagStrip({
  catalog,
  active,
  onChange,
}: {
  readonly catalog: readonly BoardTag[];
  readonly active: string;
  readonly onChange: (tag: string) => void;
}) {
  if (catalog.length === 0) {
    return null;
  }

  return (
    <fieldset className="board-tag-strip" aria-label="Filter by board tag">
      {catalog.map((tag) => {
        const on = active !== "" && sameTagName(tag.name, active);

        return (
          <button
            key={tag.id}
            type="button"
            className="board-tag-filter"
            aria-pressed={on}
            onClick={() => onChange(on ? "" : tag.name)}
          >
            <span className="board-tag-filter-face">
              <TagEmoji emoji={tag.emoji} />
              {tag.name}
            </span>
          </button>
        );
      })}
    </fieldset>
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
  catalog,
  onChange,
  onNewPost,
  canAdminister,
  automationsOpen,
  onAutomations,
  onManageTags,
}: BoardToolbarProps) {
  const users = useStore((state) => state.users);
  const active = activeFilterCount(query);
  const list = query.view === "list";
  const others = tagCounts.filter((tag) => catalogTag(catalog, tag.name) === undefined);
  const activeTag = catalogTag(catalog, query.tag)?.name ?? query.tag;

  const otherTag = (tag: BoardTagCount) => (
    <MenuRadioItem
      key={tag.name}
      checked={sameTagName(query.tag, tag.name)}
      onSelect={() => onChange({ tag: tag.name })}
    >
      {tag.name} <span className="board-menu-count tabular">({tag.count})</span>
    </MenuRadioItem>
  );

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
          <FilterChip
            label="Tag"
            value={<TagValue catalog={catalog} tag={query.tag} />}
            onClear={() => onChange({ tag: "" })}
          />
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
          <SubMenu label={`Tag: ${query.tag === "" ? "Any" : activeTag}`} icon="hash">
            <MenuGroup label="Tag">
              <MenuRadioItem checked={query.tag === ""} onSelect={() => onChange({ tag: "" })}>
                Any tag
              </MenuRadioItem>
              {catalog.length === 0 ? others.map(otherTag) : null}
            </MenuGroup>
            {catalog.length > 0 ? <MenuSeparator /> : null}
            {catalog.length > 0 ? (
              <MenuGroup label="Board tags">
                {catalog.map((tag) => (
                  <MenuRadioItem
                    key={tag.id}
                    checked={query.tag !== "" && sameTagName(query.tag, tag.name)}
                    onSelect={() => onChange({ tag: tag.name })}
                  >
                    <TagEmoji emoji={tag.emoji} />
                    {tag.name}{" "}
                    <span className="board-menu-count tabular">
                      ({postsUsing(tagCounts, tag.name) ?? 0})
                    </span>
                  </MenuRadioItem>
                ))}
              </MenuGroup>
            ) : null}
            {catalog.length > 0 && others.length > 0 ? <MenuSeparator /> : null}
            {catalog.length > 0 && others.length > 0 ? (
              <MenuGroup label="Other tags">{others.map(otherTag)}</MenuGroup>
            ) : null}
            {canAdminister ? <MenuSeparator /> : null}
            {canAdminister ? (
              <MenuItem icon="pencil" onSelect={onManageTags}>
                Manage board tags…
              </MenuItem>
            ) : null}
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
