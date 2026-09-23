# Audit log

Security-relevant actions across the workspace are recorded in an
append-only audit log (`AuditLog`), which administrators browse and export
at `/account/audit_log`. Rows are kept for one year, then pruned by
`Retention::PruneJob`.

## What is recorded

Each row stores the action name, the actor (with a name/email snapshot),
the target (with a label snapshot), a before/after `changes` payload, the
request IP and user agent, and the time. Actor and target labels are
snapshots: later renames, deactivations, and deletions never rewrite
history. There are no foreign keys from the log to users or targets.

| Area | Actions |
| --- | --- |
| Sign-in | `session.sign_in.success`, `session.sign_in.failure` (password, Google, transfer). Failures collapse per IP and email label inside a 5-minute window, with a per-IP cap of 20 rows per window; past the cap the latest row's `suppressed_count` counts the rest. |
| Two-step | `sign_in.two_factor.failure` (wrong code at the challenge), `two_factor.enable`, `two_factor.disable`, `two_factor.reset`, `two_factor.backup_codes.regenerate`. Codes and secrets never reach the log. |
| Members | `user.create` (Google-provisioned), `user.email.change`, `user.password.change`, `user.role.change`, `user.ban`, `user.unban`, `user.deactivate` |
| Google | `google.sign_in.link` (profile links and first-sign-in auto-links), `google.sign_in.link_allow`, `google.sign_in.unlink`, `google.account.connect`, `google.account.disconnect` |
| GitHub | `github.account.connect` (personal token and GitHub App OAuth), `github.account.disconnect` (members), `agent.github.connect`, `agent.github.disconnect` (agents) |
| Fizzy | `fizzy.account.connect`, `fizzy.account.disconnect` (members) |
| Account | `account.join_code.reset`, `account.settings.change`, `account.custom_styles.change` |
| Agents | `agent.create`, `agent.update`, `agent.suspend`, `agent.credential.create`, `agent.credential.revoke`, `agent.credential.reset`, `agent.grant.create`, `agent.grant.revoke`, `agent.webhook_url.change`, `agent.webhook_secret.reset`, `agent.approval.decide`, `agent.github_action.execute`, `agent.fizzy_action.execute`, `agent.kill_switch` |
| Rooms | `room.create`, `room.destroy`, `room.membership.change` |
| Boards | `board.automation.change` (tag rules and SLA timers; only actual changes) |
| Work | `work.handoff` (ownership transfers to another agent, with a summary excerpt) |
| Icons | `workspace_icon.create`, `workspace_icon.destroy` |

No action is recorded when nothing changed (re-saving an unchanged form,
re-granting an existing capability) or when the underlying operation
failed. The one deliberate exception is sign-in failures, which have no
successful operation behind them.

## Recording a new action

All writes funnel through one entry point. Call it from the controller,
service, or job that performs the action, after the operation succeeds:

```ruby
AuditLog.record!(
  action: "agent.grant.create",
  actor: Current.user,      # optional: defaults to Current.user
  target: grant,            # optional: any model; type, id, and label are snapshotted
  changes: { capability: grant.capability, room: grant.room&.name }
)
```

Conventions for new actions:

- Name actions `resource.verb(.qualifier)`, e.g.
  `agent.github_action.execute` and `agent.fizzy_action.execute` for the
  integrations' external actions: the approving human is the actor, the
  approval is the target, `changes` carries the action name, status, and
  outcome URL or message.
- Add the name to `AuditLog::ACTIONS` so it appears in the admin filter
  dropdown. Rows store plain strings, so this needs no migration.
- `changes` values SHOULD be `AuditLog.pair(before, after)` pairs, which
  the admin UI renders as "before → after"; scalar context values
  (reasons, URLs, notes) are allowed. Plain two-element arrays render as
  lists, never as pairs.
- The request IP and user agent default to `Current.request` (the user
  agent is truncated to 512 chars); jobs pass `actor:` explicitly and
  leave them blank.

## Secrets never reach the log

Callers pass explicit change hashes — never raw params — and `record!`
additionally filters every key matching passwords, tokens, secrets, keys,
credentials, and session values (see `AuditLog::SECRET_KEY_PATTERN`),
replacing them with `[FILTERED]`, including inside nested hashes and
arrays. Join codes, bot keys, credential secrets, signing secrets, OAuth
tokens, and passwords therefore cannot land in the log even if a caller
passes them by mistake. Actor and target emails stay readable: they are
the point of the log.

Three spots need more than the key filter, and summarize instead of
storing the raw value:

- Failed sign-ins store the typed email only when it matches an existing
  account or has an email shape (truncated to 254 chars); anything else
  — usually a password typed into the email field — is stored as
  `[unrecognized]`.
- Webhook URL changes store each side as its origin (scheme + host +
  non-default port) plus a short SHA-256 digest prefix of the full URL,
  so a change is visible without the secret-bearing path and query.
- Custom-styles changes store each revision as its byte size plus a
  short SHA-256 digest prefix, not the full stylesheet.

The CSV export additionally neutralizes spreadsheet formula injection:
cells starting with `=`, `+`, `-`, `@`, tab, or return are prefixed with
`'`, since actor names, room names, and change payloads are
member-controlled.

## Guarantees and limits

- **Append-only.** Persisted rows are readonly and `destroy` is refused;
  the only delete path is the one-year retention prune, which uses
  `delete_all`. Tests cover both.
- **Admins only.** The browser UI and the CSV export require a current
  administrator (`ensure_can_administer`); members get 403 and visitors
  are sent to sign in. Both responses are served `no-store`.
- **CSV export** honors the current filters and is capped at 5,000 of the
  newest matching rows. Past the cap the HTML page shows a notice and
  the downloaded filename says it is truncated.
- **Floods.** Besides the sign-in failure throttle above, recording adds
  one insert to the audited request; hot paths (message delivery, polling,
  webhooks) record nothing.
