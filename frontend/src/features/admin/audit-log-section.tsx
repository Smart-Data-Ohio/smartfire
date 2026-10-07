import { type FormEvent, useCallback, useEffect, useState } from "react";
import type { AuditLogFilters } from "../../gen/AuditLogFilters.ts";
import type { AuditLogPage } from "../../gen/AuditLogPage.ts";
import { admin } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { SettingsPage } from "../settings/settings-parts.tsx";
import { auditTime, filterValue, NO_FILTERS } from "./admin-format.ts";
import { AdministratorsOnly, adminFailure, useAdmin } from "./admin-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly page: AuditLogPage };

/** What the log shows: the filters applied and the page (`null` for the newest). */
interface Query {
  readonly filters: AuditLogFilters;
  readonly page: string | null;
}

/** A labelled select of `options`, `""` for all of them. */
function FilterSelect({
  id,
  label,
  all,
  options,
  value,
  onChange,
}: {
  readonly id: string;
  readonly label: string;
  readonly all: string;
  readonly options: readonly string[];
  readonly value: string;
  readonly onChange: (value: string) => void;
}) {
  return (
    <div className="settings-field">
      <label className="settings-label" htmlFor={id}>
        {label}
      </label>
      <select
        id={id}
        className="input settings-select"
        value={value}
        onChange={(event) => onChange(event.target.value)}
      >
        <option value="">{all}</option>
        {options.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
    </div>
  );
}

/** The filter form, seeded from the filters the log last applied. */
function Filters({
  page,
  onApply,
}: {
  readonly page: AuditLogPage;
  readonly onApply: (filters: AuditLogFilters) => void;
}) {
  const { filters } = page;
  const [actor, setActor] = useState(filters.actor ?? "");
  const [action, setAction] = useState(filters.action ?? "");
  const [targetType, setTargetType] = useState(filters.targetType ?? "");
  const [from, setFrom] = useState(filters.from ?? "");
  const [to, setTo] = useState(filters.to ?? "");

  const submit = (event: FormEvent) => {
    event.preventDefault();
    onApply({
      actor: filterValue(actor),
      action: filterValue(action),
      targetType: filterValue(targetType),
      from: filterValue(from),
      to: filterValue(to),
    });
  };

  return (
    <form className="settings-form admin-audit-filters" onSubmit={submit}>
      <div className="admin-audit-grid">
        <TextField
          label="Actor"
          value={actor}
          autoComplete="off"
          placeholder="Name or email"
          onChange={(event) => setActor(event.target.value)}
        />
        <FilterSelect
          id="admin-audit-action"
          label="Action"
          all="All actions"
          options={page.actions}
          value={action}
          onChange={setAction}
        />
        <FilterSelect
          id="admin-audit-target"
          label="Target"
          all="All targets"
          options={page.targetTypes}
          value={targetType}
          onChange={setTargetType}
        />
        <TextField
          label="From"
          type="date"
          value={from}
          onChange={(event) => setFrom(event.target.value)}
        />
        <TextField
          label="To"
          type="date"
          value={to}
          onChange={(event) => setTo(event.target.value)}
        />
      </div>
      <div className="settings-actions">
        <Button type="submit" variant="primary">
          Filter
        </Button>
        <Button variant="ghost" onClick={() => onApply(NO_FILTERS)}>
          Clear
        </Button>
        <a className="settings-link admin-export" href={page.exportUrl} download>
          Export CSV
        </a>
      </div>
    </form>
  );
}

/**
 * Audit log: security-relevant actions across the workspace, newest first, filtered by actor,
 * action, target and dates as the classic page filters them, a page at a time, with the same CSV
 * export.
 */
export function AuditLogSection() {
  const { workspace } = useAdmin();
  const [query, setQuery] = useState<Query>({ filters: NO_FILTERS, page: null });
  const [load, setLoad] = useState<Load>({ status: "loading" });

  const fetchPage = useCallback((next: Query) => {
    admin.auditLog(next.filters, next.page).then(
      (page) => setLoad({ status: "ready", page }),
      (error: Error) => {
        setLoad({ status: "error", message: error.message });
        adminFailure("Couldn't load the audit log", error);
      },
    );
  }, []);

  useEffect(() => {
    if (workspace.canAdminister) fetchPage(query);
  }, [fetchPage, query, workspace.canAdminister]);

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  const show = (next: Query) => {
    setLoad({ status: "loading" });
    setQuery(next);
  };

  const reload = () => {
    setLoad({ status: "loading" });
    fetchPage(query);
  };

  return (
    <SettingsPage
      title="Audit log"
      description="Security-relevant actions across the workspace, kept for one year."
    >
      {load.status === "ready" ? (
        <Filters
          key={JSON.stringify(load.page.filters)}
          page={load.page}
          onApply={(filters) => show({ filters, page: null })}
        />
      ) : null}
      {load.status === "loading" ? <PaneListSkeleton rows={5} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {load.status === "ready" ? (
        <>
          {load.page.exportTruncated ? (
            <p className="text-muted">
              The CSV export covers only the newest {load.page.exportLimit} matching rows.
            </p>
          ) : null}
          {load.page.entries.length === 0 ? (
            <p className="text-muted">No audit entries match these filters.</p>
          ) : (
            <section className="admin-audit-wrap" aria-label="Audit log entries">
              <table className="admin-audit-table">
                <caption className="visually-hidden">Audit log entries, newest first</caption>
                <thead>
                  <tr>
                    <th scope="col">Time</th>
                    <th scope="col">Actor</th>
                    <th scope="col">Action</th>
                    <th scope="col">Target</th>
                    <th scope="col">Changes</th>
                    <th scope="col">IP</th>
                  </tr>
                </thead>
                <tbody>
                  {load.page.entries.map((entry) => (
                    <tr key={entry.id}>
                      <td>
                        <time dateTime={entry.createdAt}>{auditTime(entry.createdAt)}</time>
                      </td>
                      <td>{entry.actor ?? "—"}</td>
                      <td>
                        <code>{entry.action}</code>
                      </td>
                      <td>{entry.target ?? "—"}</td>
                      <td className="admin-audit-changes">{entry.changes}</td>
                      <td>{entry.ipAddress ?? "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </section>
          )}
          <div className="settings-actions admin-audit-pages">
            {query.page === null ? null : (
              <Button variant="secondary" onClick={() => show({ ...query, page: null })}>
                Newest entries
              </Button>
            )}
            {load.page.nextPage === null ? null : (
              <Button
                variant="secondary"
                onClick={() => {
                  if (load.page.nextPage !== null) show({ ...query, page: load.page.nextPage });
                }}
              >
                Older entries
              </Button>
            )}
          </div>
        </>
      ) : null}
    </SettingsPage>
  );
}
