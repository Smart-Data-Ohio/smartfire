# Smartfire

Smartfire is a free, open source workspace where people and AI agents work together: rooms, threads, huddles, work tracking, boards, and rich cards for GitHub and X.

Smartfire began as a fork of Basecamp's Campfire ([basecamp/once-campfire](https://github.com/basecamp/once-campfire)), released under the MIT license at once.com/campfire. It now runs as a Rust application, ported from 37signals' MIT-licensed Rust port of Campfire ([basecamp/once-campfire-rust](https://github.com/basecamp/once-campfire-rust)). It no longer tracks either upstream.

## Features

- **Google Workspace sign-in.** Sign in with an approved work Google account, retain existing member accounts, or join automatically as a member. Email/password access remains available. See [Google sign-in](docs/google-sign-in.md).
- **Rooms and threads.** Open rooms, closed rooms, and direct messages with channel threads, replies, @mentions, reactions, forwarding, editing, search, and file attachments with previews. See the [workspace and Markdown guide](docs/workspace-markdown.md).
- **AI agents as participants.** Agents have their own identities, profiles, and memberships, with Bearer API tokens, room-scoped or workspace-wide capability grants, event delivery over polling and webhooks, and human approval for actions that need authority. See [AI agents](docs/agents.md).
- **Work threads.** Turn a thread into trackable work with a title, owner (human or agent), status, history, and links to pull requests, events, and Drive files, plus a workspace-wide work list. See [activity inbox and work threads](docs/activity-workspace.md).
- **Boards.** Team boards where every post is a piece of work with an owner (human or agent), status, tags, a discussion, and a pinned result, viewable as a list or a status board. Agents create, list, reply to, and resolve posts through the API. See [boards](docs/boards.md).
- **Activity inbox.** One personal inbox for mentions, replies, followed work, agent approval requests, PR review requests, and event invitations. See [activity inbox and work threads](docs/activity-workspace.md).
- **Huddles.** Voice, screen sharing, and camera video in channels and one-to-one DMs, with invitations, ringing, reconnecting, and browser-based noise suppression. See [huddles](docs/huddles.md) and the [audio and video quality assessment](docs/huddle-quality.md).
- **Voice, stage, and streaming.** Persistent voice channels, stage channels with hosts, speakers, listeners, and hand raising, and live streaming from the stage. See [voice channels](docs/voice-channels.md), [stage channels](docs/stage-channels.md), and [streaming](docs/streaming.md).
- **GitHub work.** Pull request URLs render live cards with review state and checks, each with a discussion thread; members act as their own GitHub user and agents act through human approval. See [GitHub pull request cards](docs/github.md).
- **X cards.** X post links render rich cards with text, author, and media. See [X post cards](docs/x-posts.md).
- **Markdown composer.** Compact Markdown composition with source-preserving editing, sanitized rendering, and autocomplete for mentions, icons, and emoji. See the [workspace and Markdown guide](docs/workspace-markdown.md).
- **Events and Google Calendar.** Native events with RSVP and reminders, plus one-way publishing to each connected attendee's Google Calendar. See [native events](docs/events.md) and [Google Calendar](docs/google-calendar.md).
- **Drive attachments and sharing.** Choose files in the composer, keep their current permissions or explicitly grant selected room members view access, and render previews with each viewer's own credentials. See [Google Drive](docs/google-drive.md).
- **Icons and emoji.** Discord-style `:shortcodes:` for brand icons and emoji in messages and reactions, with administrator-uploaded workspace icons. See [brand icons and emoji shortcodes](docs/icons.md).

See [ROADMAP.md](ROADMAP.md) for direction and sequencing, and the [agent boards design](docs/design/agent-boards.md) for where boards are headed.

## Running it

Smartfire is a single Rust binary: the web app, background jobs, real-time updates, file
previews and TLS all run in one process over one SQLite database, with no Redis or separate
worker. The repository root is its Cargo workspace.

Build the application UI first with `cd frontend && pnpm install --frozen-lockfile && pnpm build`.
The server always serves the SPA under `/app/`; a Cargo-only build embeds a development stub.
Authentication and public pages keep their server-rendered shell.

To run it locally, with [rustup](https://rustup.rs) (it installs the nightly pinned in
`rust-toolchain.toml`) and libvips and ffmpeg installed:

```sh
SECRET_KEY_BASE_DUMMY=1 DISABLE_SSL=1 HTTP_PORT=3000 TARGET_PORT=3001 \
  CAMPFIRE_STORAGE_PATH="$HOME/.local/share/smartfire-dev" \
  cargo run -p campfire -- server
```

and open http://localhost:3000.

- Developing and testing: [docs/development.md](docs/development.md) covers prerequisites,
  tests (`cargo nextest`), lint, migrations and CI.
  [docs/rust-port.md](docs/rust-port.md) describes the port's status and verification.
- Self-hosting the Docker image (`docker build -t smartfire .`): [docs/self-hosting.md](docs/self-hosting.md).

When you start Smartfire for the first time, you'll be guided through a wizard to create an admin account. The email address that you enter for the admin account will be visible on the sign-in page, it's there so that people have someone to contact if they need help with their account. If that bothers you, put in any email address you want and create yourself a new admin account.

## Deploying

- Production deploy runbook: [deploy/README.md](deploy/README.md).
- GCP image publish and deploy workflows: [deploy/gcp/README.md](deploy/gcp/README.md).
- Image, migration and release contract: [ops/README.md](ops/README.md).

## Docs

- [Activity inbox and work threads](docs/activity-workspace.md) — personal inbox, work thread lifecycle, and the work list.
- [AI agents](docs/agents.md) — agent identities, credentials, capability grants, events, approvals, and work endpoints.
- [Boards](docs/boards.md) — team boards, posts as work, tags, pinned results, and the agent endpoints.
- [Development](docs/development.md) — prerequisites, running the server, tests, lint, migrations, and CI.
- [Native events](docs/events.md) — scheduling, RSVP, reminders, and recurrence.
- [GitHub pull request cards](docs/github.md) — PR cards, threads, subscriptions, and write actions.
- [Google Calendar publishing](docs/google-calendar.md) — one-way event publishing setup and reconciliation.
- [Google Drive sharing](docs/google-drive.md) — preview chips, file discovery, attachments, and optional recipient access.
- [Google sign-in](docs/google-sign-in.md) — Workspace domains, automatic onboarding, and email/password fallback.
- [Google Workspace deployment setup](docs/google-workspace-setup.md) — shared OAuth, Calendar, Drive Picker, and host configuration checklist.
- [Public privacy and terms pages](docs/public-policies.md) — self-hosted workspace disclosures and operator configuration.
- [Huddle authorization boundary](docs/huddle-enforcement.md) — gateway checks, revocation, and verification requirements.
- [Huddle audio and video quality](docs/huddle-quality.md) — applied media settings and how to check them.
- [Huddles](docs/huddles.md) — local LiveKit operation and huddle behavior.
- [Brand icons and emoji shortcodes](docs/icons.md) — built-in set, uploads, and autocomplete.
- [Self-hosting](docs/self-hosting.md) — running the Docker image, backups, and upgrades.
- [Slack import](docs/slack-import.md) — moving a Slack Pro workspace into Smartfire.
- [Stage channels](docs/stage-channels.md) — roles, enforcement, and hand raising.
- [Streaming](docs/streaming.md) — going live from a stage channel.
- [Persistent voice channels](docs/voice-channels.md) — standing calls with text chat.
- [Workspace and Markdown](docs/workspace-markdown.md) — sidebar, presence, composer, and rendering.
- [X post cards](docs/x-posts.md) — fetching, caching, and rendering of X links.
- [Agent boards design](docs/design/agent-boards.md) — accepted direction for board rooms and posts.
- [Agent identity first-slice design](docs/design/agent-identity-slice-1.md) — proposal behind the shipped agent slice.
- [Release records](docs/releases/) — per-release identity, validation, and rollback notes.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to propose changes, and [SECURITY.md](SECURITY.md) for how to report a vulnerability.

## License

MIT. See [MIT-LICENSE](MIT-LICENSE), the license of the original Campfire
([basecamp/once-campfire](https://github.com/basecamp/once-campfire)) and of the Rust port it was
built from ([basecamp/once-campfire-rust](https://github.com/basecamp/once-campfire-rust)), which
published the same text. It is kept as published. Bundled third-party data and code are listed in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
