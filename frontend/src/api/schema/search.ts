import { Schema } from "effect";
import type { RecentSearch as GeneratedRecentSearch } from "../../gen/RecentSearch.ts";
import type { RecentSearchList as GeneratedRecentSearchList } from "../../gen/RecentSearchList.ts";
import type { RecordSearch as GeneratedRecordSearch } from "../../gen/RecordSearch.ts";
import type { SearchChip as GeneratedSearchChip } from "../../gen/SearchChip.ts";
import type { SearchOperator as GeneratedSearchOperator } from "../../gen/SearchOperator.ts";
import type { SearchResults as GeneratedSearchResults } from "../../gen/SearchResults.ts";
import type { SearchSection as GeneratedSearchSection } from "../../gen/SearchSection.ts";
import type { SearchSectionKind as GeneratedSearchSectionKind } from "../../gen/SearchSectionKind.ts";
import type { SearchSectionRow as GeneratedSearchSectionRow } from "../../gen/SearchSectionRow.ts";
import { ConversationName } from "./conversation.ts";
import { MessageId, RecentSearchId, RoomId } from "./ids.ts";
import { MessageDTO } from "./message.ts";
import type { Assert, Pinned } from "./pin.ts";
import { RoomKind } from "./room.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";

export const SearchOperator = Schema.Literals(["from", "in", "has", "before", "after", "on", "is"]);

export type SearchOperator = typeof SearchOperator.Type;

export type SearchOperatorPin = Assert<Pinned<typeof SearchOperator, GeneratedSearchOperator>>;

/** A parsed operator: the chip, and the query without it. */
export const SearchChip = Schema.Struct({
  operator: SearchOperator,
  value: Schema.String,
  token: Schema.String,
  label: Schema.String,
  removeQuery: Schema.String,
});

export type SearchChip = typeof SearchChip.Type;

export type SearchChipPin = Assert<Pinned<typeof SearchChip, GeneratedSearchChip>>;

export const SearchSectionKind = Schema.Literals(["board_posts", "work_threads", "events"]);

export type SearchSectionKind = typeof SearchSectionKind.Type;

export type SearchSectionKindPin = Assert<
  Pinned<typeof SearchSectionKind, GeneratedSearchSectionKind>
>;

/** A board post, work thread (both by thread id) or event (by event id). */
export const SearchSectionRow = Schema.Struct({
  id: Schema.Int,
  roomId: RoomId,
  roomKind: RoomKind,
  title: Schema.String,
  time: Timestamp,
  workStatus: Schema.NullOr(Schema.String),
  cancelled: Schema.Boolean,
});

export type SearchSectionRow = typeof SearchSectionRow.Type;

export type SearchSectionRowPin = Assert<
  Pinned<typeof SearchSectionRow, GeneratedSearchSectionRow>
>;

export const SearchSection = Schema.Struct({
  kind: SearchSectionKind,
  rows: Schema.Array(SearchSectionRow),
});

export type SearchSection = typeof SearchSection.Type;

export type SearchSectionPin = Assert<Pinned<typeof SearchSection, GeneratedSearchSection>>;

/**
 * `GET /api/v1/search?q=&before=`: 40 matching messages a page, oldest first, newest page
 * first; sections on the first page only. People and rooms come from the switcher, files from
 * `has:file` / `has:image`.
 */
export const SearchResults = Schema.Struct({
  query: Schema.String,
  chips: Schema.Array(SearchChip),
  messages: Schema.Array(MessageDTO),
  users: Schema.Array(User),
  conversations: Schema.Array(ConversationName),
  before: Schema.NullOr(MessageId),
  sections: Schema.Array(SearchSection),
});

export type SearchResults = typeof SearchResults.Type;

export type SearchResultsPin = Assert<Pinned<typeof SearchResults, GeneratedSearchResults>>;

export const RecentSearch = Schema.Struct({
  id: RecentSearchId,
  query: Schema.String,
  searchedAt: Timestamp,
});

export type RecentSearch = typeof RecentSearch.Type;

export type RecentSearchPin = Assert<Pinned<typeof RecentSearch, GeneratedRecentSearch>>;

/** `GET /api/v1/searches`: at most 10, most recent first. */
export const RecentSearchList = Schema.Struct({ searches: Schema.Array(RecentSearch) });

export type RecentSearchList = typeof RecentSearchList.Type;

export type RecentSearchListPin = Assert<
  Pinned<typeof RecentSearchList, GeneratedRecentSearchList>
>;

/** The body of `POST /api/v1/searches`. */
export const RecordSearch = Schema.Struct({ query: Schema.String });

export type RecordSearch = typeof RecordSearch.Type;

export type RecordSearchPin = Assert<Pinned<typeof RecordSearch, GeneratedRecordSearch>>;
