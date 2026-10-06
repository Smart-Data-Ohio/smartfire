# Production-copy cutover rehearsal (Rails -> Rust -> Rails)

Date: 2026-10-05. Everything ran on one workstation in Docker. Production was not touched: no SSH
to the VM, no deploy, no workflow dispatch, no GCP changes. The only access to anything remote was
a read-only download of the latest nightly backup object from the backup bucket.

This report contains counts and pass/fail results only. It has no message contents, names,
addresses or identifiers from the production data. The decrypted copy was kept in one private
(0700) directory and deleted when the rehearsal finished.

## Summary

| # | Check | Result |
|---|---|---|
| 1 | Session continuity across the Rails -> Rust swap (same tab, CSRF from a Rails-rendered form, 2FA session, fresh 2FA sign-in, enforced enrollment, remembered device, cable reconnect) | **PASS** (14/14) |
| 2 | Rust writes (post, edit, boost, upload, create room, change a setting), then rollback: Rails boots, renders all of it, posts and edits; schema and migrations untouched | **PASS** (16/16; schema identical before, after Rust, after rollback) |
| 3a | Rich-text bodies embedding Active Storage blob sgids | **0 records, 0 rooms** (newest: n/a) |
| 3b | Sessions that could still hold an old AES-CBC cookie | **0** (expected forced sign-outs: 0) |
| 4 | TARGET_PORT callers | **No production caller** depends on reaching the bare app on 3000 (list below) |
| 5 | Smoke, Rust vs Rails (55 pages) | **PASS**: same statuses and redirects on every page, 0 element-count differences on identical data, 0 responses >=500. Rust median p50 5.1 ms vs Rails 60.8 ms; RSS 35/90 MiB vs 628/868 MiB (idle/after smoke) |
| 6 | Background jobs with no network; outbound integrations unreachable | **PASS**: Rust ran 10 jobs, 0 left queued; no route out of the rehearsal network |

No Rust bugs found. Findings and caveats are under [Issues and observations](#issues-and-observations).

## What ran

| | |
|---|---|
| Backup | the newest nightly object under `daily/` (`smartfire-backup-20261005-173038.tar.gz.age`), format 1, age-encrypted; decrypted with the local sealed identity and checked with `deploy/backups/restore-check.sh` (SHA256SUMS, `PRAGMA integrity_check`, every blob's checksum) |
| Copy size | 16 users, 36 rooms, 290 messages, 69 attachments / 69 blobs, 16 MB of files |
| Rails image | production's image as recorded in the backup manifest: `us-central1-docker.pkg.dev/smart-data-campfire/campfire/app@sha256:5ad8d521dfd33efa5e6f9199bbf6095271cbad652f047f00f1c0a7a7a1fcabc6` (git `78b9b1546bdab4c6c1c9b8ddb94512f661289112`) |
| Rust image | `us-central1-docker.pkg.dev/smart-data-campfire/campfire/app:rust-git-e8d62adb8296f100d1de11fb9a75ca9ccf238708`, digest `sha256:b71969d5f5654fbbfdfa677e3a84f7a5950fea63777b9861d38777e0edc0ccf6` (label `net.smartdata.campfire.runtime=rust`, built from `main` at `e8d62adb`) |
| Browser | `campfire-parity-playwright:1c7609be336e` (Chromium via Playwright, built from `rust/parity/Dockerfile.playwright`) |
| Network | a Docker network created `--internal` (no route out), the app container reachable as `smartfire` |
| TLS | both runtimes ran as in production (`assume_ssl`/`force_ssl`, Secure cookies, no `DISABLE_SSL`); the driver container terminates TLS for `https://chat.rehearsal.test:8443` and pipes to `smartfire:80` |
| Secrets | a fresh rehearsal-only `SECRET_KEY_BASE` and VAPID key pair, shared by both runtimes; production's were not read |
| App env | `SKIP_TELEMETRY=1`, `APP_URL=https://chat.rehearsal.test:8443`, `WEB_CONCURRENCY=0`, `RAILS_MAX_THREADS=3`, `JOB_CONCURRENCY=1`; 2 GiB memory limit; nothing else (no mail, Slack, GitHub, Google, Fizzy, LiveKit or webhook credentials) |
| Swap | stop the container (20 s grace), start the other image on the same `/rails/storage` bind mount, same env file, same network alias: what `once update --image` does |

The same `SECRET_KEY_BASE` on both sides is what production gets too (the release keeps the env).
The fresh secret means columns production encrypted with Active Record encryption (8 TOTP
secrets, OAuth tokens, webhook secrets) can't be decrypted in the copy, so the scripts give three
users a new password and TOTP credential through Rails' own runner before the run. That is a
rehearsal artifact, not a cutover risk.

### Commands

The scripts are in `rust/reference-tools/rehearsal/`. Nothing in them names a data path or a
secret: the caller supplies a private directory.

```sh
export REHEARSAL_DIR=<private dir, 0700>          # holds the decrypted copy; delete afterwards
export RAILS_IMAGE=<production image digest from the backup manifest>
export RUST_IMAGE=<rust image>
cd rust/reference-tools/rehearsal

# Read-only fetch of the newest nightly backup (ADC token of an account with objectViewer).
TOKEN=$(gcloud auth application-default print-access-token)
API=https://storage.googleapis.com/storage/v1/b/smart-data-campfire-backups/o
OBJECT=$(curl -fsS -H "Authorization: Bearer $TOKEN" "$API?prefix=daily/" | jq -r '[.items[].name] | sort | last')
curl -fsS -H "Authorization: Bearer $TOKEN" -o "$REHEARSAL_DIR/backup.tar.gz.age" \
  "$API/$(jq -rn --arg o "$OBJECT" '$o|@uri')?alt=media"

./rehearsal.sh prepare "$REHEARSAL_DIR/backup.tar.gz.age" <age identity file>
./run-all.sh A_ID B_ID C_ID     # A, B: members sharing a room (A is the smoke viewer); C: enrolls 2FA on Rust
python3 compare.py "$REHEARSAL_DIR/work" rails rust --markdown
python3 counts.py "$REHEARSAL_DIR/base/db/production.sqlite3" 2025-08-15
./rehearsal.sh teardown && rm -rf "$REHEARSAL_DIR"
```

`run-all.sh` resets the working copy, starts Rails, prepares the users, picks the pages to smoke
(`targets.py`, by id only), probes egress, smokes Rails, swaps to Rust, smokes Rust, reruns both
on one reused session, drives `scenario.mjs` through Rails -> Rust -> Rails (`run-scenario.sh`),
smokes Rails after the rollback, compares schemas, counts jobs, scans the logs, and writes
`work/summary.txt`. It prints counts and pass/fail only.

## 1. Session continuity (Rails -> Rust)

One Chromium instance stayed up across both swaps. A's tab was signed in on Rails (password +
TOTP, "remember this device"), B in a second context; the containers were then swapped under them.

| Step | Result |
|---|---|
| Sign in A on Rails, password + TOTP, remember device | PASS |
| A's room page renders on Rails with a live cable; profile form open | PASS |
| Sign in B on Rails (2FA) | PASS |
| Rails issued the remembered-device cookie (`two_factor_remember`) | PASS |
| A's open tab: cable reconnects to Rust without a reload | PASS (15.8 s after the swap began; Rails needs ~5-6 s to stop, Rust ~1 s to start, the rest is ActionCable's reconnect backoff) |
| Composer rendered by Rails posts to Rust (CSRF token from Rails accepted) | PASS (200) |
| Profile form rendered by Rails submits to Rust (setting change) | PASS (302, saved) |
| B posts on Rust; the message reaches A's open tab live | PASS |
| A stays signed in after a reload on Rust | PASS |
| B's 2FA session from Rails stays signed in on Rust | PASS |
| Fresh 2FA sign-in for B on Rust | PASS |
| Remembered-device cookie issued by Rails skips the TOTP challenge on Rust | PASS |
| C (no credential) signs in on Rust and completes enforced 2FA enrollment | PASS |
| No 5xx or page errors on Rust outside the swap window | PASS |

## 2. Rust writes, then rollback to Rails

| Step | Result |
|---|---|
| Rust: post two messages | PASS (200, 200) |
| Rust: edit a message | PASS (302, edit applied) |
| Rust: boost a message | PASS (200) |
| Rust: upload an image | PASS (200), and Rust serves it (200) |
| Rust: create a room | PASS (200, new room) |
| Rust: change a setting (profile, item above) | PASS |
| Swap back: A's cable reconnects to Rails | PASS (19.9 s after the swap began) |
| A (cookies last written by Rust) stays signed in on Rails | PASS |
| Rails renders the room with every Rust write (posts, the composer post, the boost, the image) | PASS |
| Rails serves the file Rust uploaded | PASS (200) |
| Rails renders the room Rust created | PASS |
| Rails shows the setting Rust saved | PASS |
| Rails posts, edits its own message, and edits a message Rust created | PASS (200, 302, 302) |
| B's session created on Rust is valid on Rails | PASS |
| C's session created on Rust is valid on Rails | PASS |
| C signs in on Rails with the TOTP secret Rust encrypted (Active Record encryption round trip) | PASS |
| No 5xx or page errors on Rails before or after, outside the swap windows | PASS |
| Smoke on Rails after rollback (reused session, 55 pages) | PASS (53x200, 2x302, 0 >=500) |
| Schema + `schema_migrations` before, after Rust, after rollback vs the pristine copy | identical (129 migrations, latest `20261003180000`); Rails' `db:prepare` ran no migration on reboot |

Each swap window showed about one failed request in the browser (the request in flight when the
container stopped). That is the expected cost of a stop/start swap and the same for both directions.

## 3. Prevalence counts

`counts.py` reads the restored database read-only and prints counts only.

**(a) Rich-text bodies embedding Active Storage blob sgids** (what the Rust port renders as the
missing-attachment marker, README "Known differences").

| | |
|---|---|
| Rich-text records | 261 |
| `<action-text-attachment>` tags | 26: 25 user mentions (sgid names `User`), 1 content attachment with no sgid (a link-preview card) |
| Records embedding an `ActiveStorage::Blob` sgid | **0** |
| Distinct rooms affected | **0** |
| Newest affected `created_at` | n/a |

Files in production are message attachments (`active_storage_attachments` on `Message`, 69 of
them), which both runtimes render; no rich-text body embeds a blob. The smoke also counted the
missing-attachment marker (`☒`) on every page on both runtimes: 0 everywhere.

**(b) Old AES-CBC cookies.** Rails switched encrypted cookies to AES-256-GCM with `load_defaults
7.0`; this repository has had `config.load_defaults 7.0` or later since its first commit
(2025-08-15), and production started on 2026-09-09.

| | |
|---|---|
| Sessions | 21 (oldest created 2026-09-09) |
| Unexpired now (members never expire; admins after 7 idle days) | 21 |
| Unexpired and last active before the GCM switch | **0** |

No session, and so no browser cookie, predates GCM. Estimated forced sign-outs from the missing
CBC fallback: **0**. The scenario confirmed real Rails-issued cookies (`_campfire_session`,
`session_token`, `two_factor_remember`) are read by Rust and Rust-issued ones by Rails.

## 4. TARGET_PORT callers

In the Rust image, `campfire server` runs the front (HTTP_PORT, 80) and the app on TARGET_PORT
(3000), and the app listener binds TARGET_BIND, default `127.0.0.1`. In the Rails image Puma
listens on 3000 on all interfaces behind Thruster. The rehearsal confirmed the difference: from the
Docker network, `http://smartfire:3000/up` was reachable on Rails and unreachable on Rust.

| Caller | Uses | Affected by the loopback bind? |
|---|---|---|
| `deploy/gcp/campfire-release.sh` | health checks on `https://$host/up` (front); the Rails rollback rehearsal probes `http://127.0.0.1:3000/up` *inside* the previous Rails image | No (front port; the 3000 probe only runs against Rails) |
| everything else in `deploy/`, `.github/`, `config/`, `bin/`, `script/`, `Procfile`, root `Dockerfile` | none | - |
| `script/livekit-gateway` | defaults `GATEWAY_CAMPFIRE_URL` to `http://127.0.0.1:3000` | No: production's `deploy/huddles/compose.yaml` sets `https://chat.smartdata.net` |
| `rust/Dockerfile` | comment; `bin/boot` runs `campfire server` | - |
| `rust/crates/kit/src/front/config.rs` | reads TARGET_PORT (default 3000) and TARGET_BIND (default 127.0.0.1) | defines it |
| `rust/crates/kit/src/front.rs` | starts the app listener on TARGET_BIND:TARGET_PORT; warns and skips if TARGET_PORT equals HTTP_PORT | defines it |
| `rust/crates/campfire/src/config.rs`, `app.rs` | comments | - |
| `rust/crates/campfire/src/app/security_tests.rs`, `rust/crates/kit/tests/front.rs` | tests of the bind | - |
| `rust/bench/run`, `rust/bench/lib/benchlib.py`, `rust/bench/pi-100k/run100k.sh` | set TARGET_PORT=PORT+1 with host networking | No, except benchlib's `proxy-direct` mode (issue 2) |
| `rust/parity/bin/reference`, `rust/reference-tools/` browser scripts | set ports for local runs | No |
| `rust/README.md`, `rust/ops/README.md` | documentation | - |

## 5. Smoke and performance

55 pages chosen by id from the copy (`targets.py`): home, profile, sessions, sidebar, search
(empty and a common word), activity, saved, scheduled, work, switcher, account, users, agents,
room categories, 14 rooms (open, closed, direct, a board), room edit, members, files, pins, board
automations, 5 user profiles and 2 cards, events and threads (index and show). Each page was
fetched with no redirects followed: one warm-up, then 10 samples, by a signed-in member over the
TLS front.

| | Rails | Rust | Rails after rollback |
|---|---|---|---|
| Statuses | 53x200, 2x302 | 53x200, 2x302 | 53x200, 2x302 |
| Responses >=500 | 0 | 0 | 0 |
| Status or redirect-target mismatches vs Rails | - | 0 | - |
| Element-count differences (links, forms, inputs, buttons, images, turbo frames, cable streams, messages, templates, scripts, controllers, CSRF tokens, missing-attachment markers) on identical data | - | 0 | - |

The first Rails/Rust pass showed count differences on two pages, `/users/me/sessions` and
`/activity`. They came from the extra sign-in between the runs: each sign-in adds a session row and
a "new sign-in" activity item. Rerunning both runtimes on one reused session gave 0 differences.

**Latency** (client-side, including TLS through the front; per page p50 of 10 samples):

| | Rails | Rust |
|---|---|---|
| Median of per-page p50 | 60.8 ms | 5.1 ms |
| Sum of per-page p50 (one pass over all 55 pages) | 4031 ms | 390 ms |
| Worst page p95 (a busy room) | 395.0 ms | 20.0 ms |

**Memory and startup** (`docker stats`, 2 GiB limit; Rails runs Puma, Thruster, Redis, resque-pool and the periodic runner in its container, Rust one process):

| | Rails | Rust |
|---|---|---|
| RSS idle after boot | 627.8 MiB | 34.7 MiB |
| RSS after the smoke | 868.2 MiB | 90.2 MiB |
| Container start to `/up` 200 | ~5.4-6.2 s | ~0.7-1.3 s (includes ~1 s of probe overhead) |

<details><summary>Per-route latency (median p50 across the route's pages, worst p95)</summary>

| Route | Pages | Rails status | Rust status | Rails p50 ms | Rust p50 ms | Rails p95 ms | Rust p95 ms |
|---|---|---|---|---|---|---|---|
| `/` | 1 | 302 | 302 | 47.8 | 3.4 | 54.1 | 5.8 |
| `/users/me/profile` | 1 | 200 | 200 | 74.7 | 7.8 | 91.2 | 8.7 |
| `/users/me/sessions` | 1 | 200 | 200 | 54.8 | 4.8 | 74.1 | 7.3 |
| `/users/me/sidebar` | 1 | 200 | 200 | 67.3 | 5.9 | 76.6 | 7.3 |
| `/searches` | 1 | 200 | 200 | 54.1 | 5.3 | 58.3 | 6.8 |
| `/searches?q=<common word>` | 1 | 200 | 200 | 165.6 | 14.1 | 276.3 | 17.6 |
| `/activity` | 1 | 200 | 200 | 57.0 | 5.4 | 69.4 | 5.7 |
| `/saved` | 1 | 200 | 200 | 52.1 | 4.1 | 57.9 | 4.8 |
| `/scheduled_messages` | 1 | 200 | 200 | 53.0 | 4.6 | 58.1 | 8.3 |
| `/work` | 1 | 200 | 200 | 53.5 | 4.7 | 59.9 | 5.8 |
| `/switcher` | 1 | 200 | 200 | 52.5 | 3.1 | 53.7 | 4.5 |
| `/account/edit` | 1 | 200 | 200 | 59.1 | 6.0 | 77.5 | 7.6 |
| `/account/users` | 1 | 200 | 200 | 52.2 | 3.5 | 62.0 | 5.1 |
| `/users` | 1 | 200 | 200 | 56.0 | 5.0 | 64.9 | 6.7 |
| `/agents` | 1 | 200 | 200 | 52.0 | 4.8 | 58.6 | 6.8 |
| `/room_categories` | 1 | 200 | 200 | 45.4 | 2.6 | 46.8 | 2.9 |
| `/rooms/:id` | 14 | 200 | 200 | 82.8 | 12.8 | 395.0 | 20.0 |
| `/rooms/opens/:id/edit` | 2 | 200 | 200 | 60.4 | 5.5 | 69.8 | 10.0 |
| `/rooms/:id/members` | 1 | 200 | 200 | 57.8 | 2.9 | 58.8 | 4.2 |
| `/rooms/:id/files` | 1 | 200 | 200 | 62.3 | 4.6 | 74.8 | 7.4 |
| `/rooms/:id/pins` | 1 | 200 | 200 | 58.2 | 4.4 | 62.9 | 6.0 |
| `/rooms/boards/:id` | 1 | 302 | 302 | 48.0 | 2.2 | 49.4 | 7.1 |
| `/rooms/boards/:id/automations` | 1 | 200 | 200 | 59.1 | 5.1 | 64.1 | 6.7 |
| `/users/:id` | 5 | 200 | 200 | 59.8 | 4.4 | 78.6 | 8.7 |
| `/users/:id/card` | 2 | 200 | 200 | 50.7 | 2.7 | 55.1 | 4.3 |
| `/rooms/:id/events` | 3 | 200 | 200 | 60.8 | 4.7 | 72.1 | 6.6 |
| `/rooms/:id/events/:id` | 3 | 200 | 200 | 61.2 | 4.2 | 67.2 | 7.2 |
| `/rooms/:id/threads` | 2 | 200 | 200 | 54.8 | 4.0 | 60.5 | 6.1 |
| `/rooms/:id/threads/:id` | 3 | 200 | 200 | 72.2 | 9.2 | 80.0 | 10.9 |

</details>

## 6. Background jobs and outbound integrations

| Check | Result |
|---|---|
| Egress from the rehearsal network (`http://1.1.1.1`, `https://www.google.com`) | unreachable |
| Egress from inside the Rails container (`https://oauth2.googleapis.com`) | unreachable |
| `background_jobs` in the restored copy | empty |
| Jobs Rust performed | 10: at first boot 1 (`Retention::PruneJob`); during the scenario 9 (5 `Room::PushMessageJob`, 2 `ActiveStorage::AnalyzeJob`, 1 `Message::QuoteCardsRefreshJob`, 1 `Retention::PruneJob`) |
| Jobs left in `background_jobs` before the rollback | 0 |
| Rust ERROR/WARN/panic lines | 1 ERROR: `An unauthorized connection attempt was rejected`, from the 2FA-setup page's cable connection. Rails rejects the same connection (`app/channels/application_cable/connection.rb`) and ActionCable logs it at error level too. Not a bug. |
| Rails 5xx completions / FATAL lines (all three Rails runs) | 0 / 0 |
| Web Push | the copy has 7 push subscriptions; neither runtime logged a Web Push delivery or error (push services are unreachable from the network either way) |
| Rails' Redis (Resque) in the rehearsal | empty (production's Redis AOF isn't in the backup) |

No integration credentials were set, so mail, Slack, GitHub, Google, Fizzy, LiveKit and webhooks
were disabled as well as unreachable.

## Issues and observations

No Rust bugs were found. These are the things worth knowing before the real cutover.

1. **Rails 500 at `/rooms/:id/settings` (pre-existing, Rails side).** `config/routes.rb` line 236
   declares `resource :settings, only: :show` inside rooms, but there is no
   `Rooms::SettingsController`; nothing links to it. Already recorded as a missing controller in
   `rust/crates/routes/src/tests.rs`. Repro: sign in on Rails, `GET /rooms/<any id>/settings` ->
   500 (`ActionDispatch::MissingController: uninitialized constant Rooms::SettingsController`). The smoke uses `/rooms/opens/:id/edit` instead.
2. **The bare app on 3000 isn't reachable from outside the Rust container (by design).** Only
   matters to `rust/bench/lib/benchlib.py`'s `proxy-direct` mode, which publishes container port
   3000 without setting `TARGET_BIND`. Repro: `docker run -d -p 127.0.0.1:3999:3000 <rust image>`
   with a seed, then `curl http://127.0.0.1:3999/up` fails (nothing listens on the
   container's external interface); with `-e TARGET_BIND=0.0.0.0` it should answer 200 (not run here;
   the rehearsal saw the same unreachability from the Docker network). Tooling fix, not a product issue.
3. **Queued jobs don't cross the runtime boundary.** Rails keeps pending jobs in Resque (Redis,
   AOF under `/rails/storage`); Rust in the SQLite `background_jobs` table. Anything still queued in
   Redis at the swap is not run by Rust, and Rust rows unfinished at a rollback are not run by
   Rails. In the rehearsal both queues were empty at each swap (Rust left 0 rows). For the real
   cutover: swap at a quiet moment, and check the Resque queue length is 0 just before (scheduled
   messages are rows in the database, not queue entries, so they carry over).
4. **Swap downtime.** About one in-flight request failed per swap, and open tabs reconnected their
   cable 16-20 s after the swap began (mostly ActionCable's client backoff). Expected for a
   stop/start swap; worth a heads-up to users.
5. **Fresh-secret artifact.** Columns encrypted under production's secret (TOTP secrets, OAuth
   tokens, webhook secrets) were unreadable in the rehearsal because a fresh secret was used. The
   real cutover keeps the same env and secret, and the scenario showed Rails and Rust read each
   other's encrypted TOTP secrets under a shared secret.
6. **The backup excludes the Redis AOF.** By design (`deploy/backups/campfire-backup.sh`), so
   pending Resque jobs are not recoverable from a backup either way; noted for completeness.

## Cleanup

After the run: the app container and the rehearsal network were removed, the private data
directory (decrypted backup, working copy, browser state, logs) was deleted, and so was the
encrypted download. No Docker volumes were created (bind mounts only).
