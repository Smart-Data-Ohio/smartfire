import { Link } from "@tanstack/react-router";
import { useId } from "react";
import type { GuideEntry } from "../../gen/GuideEntry.ts";
import { useAppearance } from "../../lib/appearance.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { PageFrame } from "../destinations/page-frame.tsx";
import { BodyHtml } from "../messages/body-html.tsx";
import { PageLoading } from "../shell/page-loading.tsx";
import { setWelcomeDismissed } from "./welcome-actions.ts";
import { RoomAction } from "./welcome-room.tsx";
import { hasWelcomeCard, useWelcome } from "./welcome-store.ts";
import "./welcome.css";

function Entry({ entry }: { readonly entry: GuideEntry }) {
  const titleId = useId();

  return (
    <li className="guide-entry">
      <article className="guide-entry-card" aria-labelledby={titleId}>
        <h2 id={titleId} className="guide-entry-title">
          {entry.emoji === null ? null : (
            <span className="guide-entry-emoji" aria-hidden="true">
              {entry.emoji}
            </span>
          )}
          {entry.title}
        </h2>
        {entry.bodyHtml === null ? null : (
          <BodyHtml html={entry.bodyHtml} className="message-body guide-entry-body" />
        )}
        {entry.room === null ? null : (
          <div className="guide-entry-room">
            <Icon name="hash" size={16} className="guide-entry-room-glyph" />
            <span className="guide-entry-room-name">{entry.room.name}</span>
            <RoomAction roomId={entry.room.id} name={entry.room.name} joined={entry.room.joined} />
          </div>
        )}
      </article>
    </li>
  );
}

/**
 * `/app/guide`: the server guide, read-only for everyone, in the administrators' order. Room
 * links the viewer can't reach were left out by the server. Administrators get an Edit link; a
 * dismissed welcome card can be brought back to Home from here.
 */
export function GuideRoute() {
  const { status, welcome } = useWelcome();
  const { welcomeDismissed } = useAppearance();

  const edit =
    welcome?.canAdminister === true ? (
      <Link
        to="/admin/workspace"
        hash="guide"
        className="button"
        data-variant="ghost"
        data-size="sm"
        aria-label="Edit the server guide"
      >
        <Icon name="pencil" size={14} />
        Edit
      </Link>
    ) : undefined;

  return (
    <PageFrame title="Server guide" icon="map-pin" back tools={edit}>
      {welcome === null ? (
        status === "failed" ? (
          <p className="guide-empty text-muted">Couldn't load the server guide.</p>
        ) : (
          <PageLoading />
        )
      ) : (
        <div className="guide-scroll">
          <div className="guide-column">
            {welcome.guide.length === 0 ? (
              <p className="guide-empty text-muted">There's nothing in the server guide yet.</p>
            ) : (
              <ol className="guide-list">
                {welcome.guide.map((entry, index) => (
                  // biome-ignore lint/suspicious/noArrayIndexKey: entries have no id; the list is replaced whole on every save
                  <Entry key={index} entry={entry} />
                ))}
              </ol>
            )}
            {welcomeDismissed && hasWelcomeCard(welcome) ? (
              <p className="guide-reopen">
                <Button variant="link" size="sm" onClick={() => setWelcomeDismissed(false)}>
                  Show the welcome on Home again
                </Button>
              </p>
            ) : null}
          </div>
        </div>
      )}
    </PageFrame>
  );
}
