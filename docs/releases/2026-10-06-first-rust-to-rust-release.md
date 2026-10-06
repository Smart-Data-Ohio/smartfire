# First Rust-to-Rust release — October 6, 2026

The first production release that moved one Rust image to another, through the Rust-native release path, is live at https://chat.smartdata.net. Production had already switched from Rails to the Rust image on October 5 (release `20261005-e8d62ad-37389550885`); this release replaced that image with one built from the commit that taught the release script to release Rust to Rust and to migrate the live database with `campfire db-migrate`. It carried no schema migration, so users see no change.

## Release identity

- Application source: `6b032694f9b82544edd14446e29a19dbbc82cbac` (the merge of PR #258, "deploy: Rust-to-Rust releases with a Rust-native migration path", on `main`).
- Image: `us-central1-docker.pkg.dev/smart-data-campfire/campfire/app@sha256:f0914744852d00a92a949406a8ba924ec33e3a24a1fafc40aa3df674135aec2c`, published as tag `rust-git-6b032694f9b82544edd14446e29a19dbbc82cbac`.
- Previous application source: `e8d62adb8296f100d1de11fb9a75ca9ccf238708`.
- Previous image: `us-central1-docker.pkg.dev/smart-data-campfire/campfire/app@sha256:b71969d5f5654fbbfdfa677e3a84f7a5950fea63777b9861d38777e0edc0ccf6`.
- Environment: `production` (`campfire`, `us-central1-a`, host `chat.smartdata.net`).
- Released by the [Deploy to GCP](../../.github/workflows/deploy-gcp.yml) workflow, [run 37424709970](https://github.com/Smart-Data-Ohio/smartfire/actions/runs/37424709970), after a production dry run ([run 37424129519](https://github.com/Smart-Data-Ohio/smartfire/actions/runs/37424129519)). Release label `20261006-6b03269-37424709970`.
- The existing ONCE application and storage volume were upgraded in place with `once update chat.smartdata.net --image …@sha256:f091… --auto-update=false`. `--env` was omitted, so ONCE kept the whole existing environment map, session-signing and web-push keys. Automatic upstream updates remain disabled.

## What shipped

Relative to the previous production image:

- PR #258 — Rust-native schema migrations (SQL files in `rust/crates/db/migrations`, applied by `campfire db-migrate`; boot stays strict) and a release script that only releases Rust to Rust, rehearsing `db-migrate`, `db-check` and the additive verifier on a copy of the frozen database before migrating the live one.
- PR #259 — the Rust port's static inputs (assets, JavaScript, `public/`, fixtures) moved into `rust/`.
- PR #260, PR #254 — Rust CI test-count enforcement, job gating and wall-time reductions.
- PR #256 — Cranelift for dev/test builds and faster linking (release builds stay on LLVM).
- PR #255 — the production-copy cutover rehearsal record.

## Validation

- [Rust checks on the application revision](https://github.com/Smart-Data-Ohio/smartfire/actions/runs/37421761806) (the `Rust` push run for `6b032694f`) passed.
- Preflight confirmed the candidate image's `GIT_REVISION` matched the requested revision and that the image is `linux/amd64`.
- Before the cutover, the candidate image migrated a copy of the frozen database in a network-isolated throwaway container and its schema check accepted the result: `PASSED: no migrations; 93 preexisting tables preserved; 0 tables, 0 columns`.
- Migrations applied to the live database with `campfire db-migrate` while writes were frozen: none.
- Writes were frozen at `2026-10-06T06:40:11Z` and reopened at `2026-10-06T06:41:28Z`.
- After the cutover, health `GET https://chat.smartdata.net/up` returned `200`, the running digest and the storage volume `once-app-once-campfire.b18a37` matched, the environment keys `APP_URL`, `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `GOOGLE_CLOUD_PROJECT_NUMBER`, `GOOGLE_PICKER_API_KEY`, `GOOGLE_SIGN_IN_DOMAINS`, `LIVEKIT_API_KEY`, `LIVEKIT_API_SECRET`, `LIVEKIT_GATEWAY_SECRET`, `LIVEKIT_INTERNAL_URL`, `LIVEKIT_URL` and `WEB_CONCURRENCY` were preserved (names only), all 69 pre-existing uploaded files were unchanged, and the Rust server process was present.
- Host preparation: `1024 MB` swap at `/swapfile` (active `true`, created by this release `false`), `vm.swappiness=10`.
- Open Roles timer restored to `enabled=enabled active=active`; temporary registry credentials removed: `true`.

## Timeline

All times UTC.

| Time | Step |
| --- | --- |
| 06:40:11 | Open Roles timer paused; database snapshot with the SQLite backup API. |
| 06:40:12 | Application stopped; writes frozen. Frozen copy of the database kept. |
| 06:40:13–06:40:14 | ONCE archive taken; migration rehearsal on a database copy inside the candidate image. |
| before 06:41:15 | Boot-disk snapshot reached `READY`. |
| 06:41:20–06:41:21 | `campfire db-migrate` on the stopped live database: no migrations. |
| 06:41:21 | `once update` to the pinned digest. |
| 06:41:23 | New container `once-app-once-campfire.b18a37-93caa3` returned health 200; post-cutover checks passed. |
| 06:41:28 | Writes reopened; Open Roles timer restored. |

Writes were frozen for 77 seconds.

## Recovery checkpoints

- Protected coherent app and feed checkpoint on the app VM: `/var/backups/campfire-20261006-6b03269-37424709970`, holding the frozen copy of the live database (`frozen-live/`), the copy the cutover left (`migrated-live/`), `before.once.tar.gz`, the host/feed archive and the per-phase result JSON files. Treat its contents as secret: they contain user data and application keys.
- Boot-disk snapshot: `campfire-before-20261006-6b03269-37424709970-r37424709970`, verified `READY` while writes were paused.
- Retained old image tag: `campfire-rollback:before-20261006-6b03269-37424709970`.
- The coherent app and host/feed archive SHA-256s are in the run's job summary and in the state directory's `freeze-result.json`.

The finish phase pruned the release directories of the two older releases (`campfire-20261005-e8d62ad-37388924178` and `campfire-20261004-78b9b15-37202819510`).

Chat has reopened. See [the automated release pipeline](../../deploy/gcp/README.md) and [the deployment procedure](../../deploy/README.md) for rollback behavior.
