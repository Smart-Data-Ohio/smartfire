# Navigation: switcher, shortcuts, unread and sidebar

Four connected pieces for moving around the workspace: the quick
switcher, global keyboard shortcuts, the "New messages" divider with
jump-to-unread, and sidebar organizing (favourites, mute, categories).

## Quick switcher

**Ctrl+K** (or **⌘+K**) anywhere — except inside the huddle device
menus — opens a modal combobox over your rooms (channels, DMs, group
DMs, voice, Stage, boards), people, and recent threads. Type to fuzzy
filter; **↑**/**↓** move, **Enter** opens, **Esc** closes. With an
empty query your recent rooms lead. Choosing a person opens a DM with
them, creating it when none exists.

One JSON endpoint (`GET /switcher.json`) serves the whole switcher in
a constant number of queries. It lists visible memberships only, so
rooms you cannot access — and rooms you hid — never appear. Recents
are this browser's visited rooms, kept in local storage per user.

## Keyboard shortcuts

Press **?** (outside any input) for the full sheet. The globals:

- **Ctrl/⌘+K** — quick switcher
- **Alt+↑/↓** — previous / next room in sidebar order
- **Alt+Shift+↑/↓** — previous / next unread room
- **Esc** — mark the current room read
- **?** — the shortcut sheet

Unchorded shortcuts never fire while typing, and neither do
Alt+↑/↓ (macOS Option+↑/↓ moves by paragraph in text). Ctrl/⌘+K
works from the composer, except while composing (IME) or over
another open modal. Esc closes open dialogs, menus and panels
first, and only marks the room read when nothing is open. The
sheet also lists every existing
shortcut: message-list movement and menus, composer sending and
formatting, the emoji and Drive pickers, and panel closing.

Note: Ctrl+K used to insert a link in the Markdown composer. It now
opens the switcher everywhere; bold (**Ctrl+B**) and italic
(**Ctrl+I**) keep their composer shortcuts.

## New messages divider

Each membership remembers its last-read message (`last_read_message_id`,
set whenever the room is read: opening it, the reads endpoint, muting,
or presence). Opening a room renders a **New messages** divider above
the first unread message. With more than five unread the room scrolls
to the divider; with fewer it scrolls to the bottom as before. The
room always opens on its usual page: when the first unread fell off
it, no divider renders and the **Jump to unread** pill links to an
anchored page that shows it. Otherwise scrolling the divider
off-screen reveals the pill, which scrolls back to it.

**Mark unread** in a message's menu moves the pointer to just before
that message, so the divider lands above it on the next visit. It
applies to room messages only; thread replies keep their own read
state. Rows that predate the pointer fall back to the `unread_at`
stamp for the boundary.

## Sidebar: favourites, mute, categories

Right-click (or **Shift+F10**) any sidebar row for the room menu:

- **Favourites**: star any room into the **Favourites** section at the
  top. Favourited rooms live only there. Reorder by dragging within
  the section, or with **Move up** / **Move down** in the room's menu.
- **Mute**: an involvement level between `nothing` and `mentions`,
  also reachable from the header bell (which cycles through it). Muted
  rooms dim, go unread only when you are mentioned, and push only for
  mentions — no reply-author push, no reply or thread-activity inbox
  items. Muting clears stale unread state. Unmuting from the menu
  restores the room's default involvement.
- **Categories**: user-defined collapsible sections for channels.
  Create one from **New category** under Channels, then drag channels
  in (or assign them from the room menu), collapse, rename, or delete
  it. Deleting returns its channels to Channels. Collapsed state
  persists per user. Categories hold channels only, and favourited
  rooms stay in Favourites until unfavourited.

All three store per membership (`favorite_position`, `involvement`,
`room_category_id`) or in the per-user `room_categories` table, and
the sidebar renders them in a constant number of queries.

## Endpoints

| Method | Path | Purpose |
| ------ | ---- | ------- |
| GET | `/switcher.json` | Switcher rooms, people, threads |
| POST | `/rooms/:id/read` | Mark read (Esc) |
| DELETE | `/rooms/:id/read?message_id=` | Mark unread from a message |
| POST / DELETE | `/rooms/:id/favorite` | Favourite / unfavourite |
| PATCH | `/rooms/:id/favorite` (`position`) | Move a favourite |
| POST / PATCH / DELETE | `/room_categories` | Manage categories |
| GET | `/room_categories.json` | List categories for the room menu |
| PATCH | `/rooms/:id/category_assignment` | Assign a channel to a category |
| PUT | `/rooms/:id/involvement` (`muted`) | Mute / unmute |
