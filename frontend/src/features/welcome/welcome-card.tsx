import { Link } from "@tanstack/react-router";
import { useId, useRef } from "react";
import type { WorkspaceWelcome } from "../../gen/WorkspaceWelcome.ts";
import { useStore } from "../../store/store.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { BodyHtml } from "../messages/body-html.tsx";
import { setWelcomeDismissed } from "./welcome-actions.ts";
import { RoomAction } from "./welcome-room.tsx";
import "./welcome.css";

interface WelcomeCardProps {
  readonly welcome: WorkspaceWelcome;
  /** Tighter, for the top of the conversation list on phones. */
  readonly compact?: boolean;
}

/** The first focusable thing after `element` in its container, once the card has gone. */
function nextFocusable(element: HTMLElement | null): HTMLElement | null {
  let sibling = element?.nextElementSibling ?? null;

  while (sibling !== null) {
    const target = sibling.matches("a[href], button")
      ? sibling
      : sibling.querySelector("a[href], button");

    if (target instanceof HTMLElement) {
      return target;
    }

    sibling = sibling.nextElementSibling;
  }

  return null;
}

/**
 * The workspace's welcome (Discord's welcome screen, without the onboarding): its logo, name and
 * description, the administrators' message, and the rooms they suggest, each with Open or Join.
 * Only rooms this viewer can reach ever arrive here. Dismissing it is saved for the person, and
 * focus moves on to whatever follows the card.
 */
export function WelcomeCard({ welcome, compact = false }: WelcomeCardProps) {
  const headingId = useId();
  const roomsId = useId();
  const ref = useRef<HTMLElement>(null);

  const logo = useStore(
    (state) => state.boot?.account.logoStillUrl ?? state.boot?.account.logoUrl ?? null,
  );

  // In the list on phones the sidebar's own "Server guide" row sits just below the card.
  const guideLink = welcome.guide.length > 0 && !compact;

  const dismiss = () => {
    const next = nextFocusable(ref.current);

    setWelcomeDismissed(true);
    requestAnimationFrame(() => {
      if (next?.isConnected === true) {
        next.focus();
      }
    });
  };

  return (
    <section
      ref={ref}
      className="welcome-card"
      data-compact={compact || undefined}
      aria-labelledby={headingId}
    >
      <header className="welcome-card-head">
        {logo === null ? (
          <span className="welcome-card-logo welcome-card-monogram" aria-hidden="true">
            {welcome.name.trim().charAt(0).toUpperCase() || "S"}
          </span>
        ) : (
          <img className="welcome-card-logo" src={logo} alt="" width={40} height={40} />
        )}
        <div className="welcome-card-titles">
          <h2 id={headingId} className="welcome-card-title">
            Welcome to {welcome.name}
          </h2>
          {welcome.description === "" ? null : (
            <p className="welcome-card-description">{welcome.description}</p>
          )}
        </div>
        <IconButton
          icon="x"
          size="sm"
          label="Dismiss welcome"
          className="welcome-card-dismiss"
          onClick={dismiss}
        />
      </header>
      {welcome.messageHtml === null ? null : (
        <BodyHtml html={welcome.messageHtml} className="message-body welcome-card-message" />
      )}
      {welcome.rooms.length === 0 ? null : (
        <div className="welcome-card-rooms">
          <h3 id={roomsId} className="welcome-card-subtitle">
            Suggested channels
          </h3>
          <ul className="welcome-room-list" aria-labelledby={roomsId}>
            {welcome.rooms.map((room) => (
              <li key={room.roomId} className="welcome-room">
                <span className="welcome-room-glyph" aria-hidden="true">
                  {room.emoji ?? <Icon name="hash" size={16} />}
                </span>
                <span className="welcome-room-text">
                  <span className="welcome-room-name">{room.name}</span>
                  {room.description === "" ? null : (
                    <span className="welcome-room-description">{room.description}</span>
                  )}
                </span>
                <RoomAction roomId={room.roomId} name={room.name} joined={room.joined} />
              </li>
            ))}
          </ul>
        </div>
      )}
      {guideLink || welcome.canAdminister ? (
        <footer className="welcome-card-foot">
          {!guideLink ? null : (
            <Link to="/guide" className="welcome-card-link">
              <Icon name="map-pin" size={14} />
              Server guide
            </Link>
          )}
          {welcome.canAdminister ? (
            <Link to="/admin/workspace" hash="welcome" className="welcome-card-link">
              <Icon name="pencil" size={14} />
              Edit welcome
            </Link>
          ) : null}
        </footer>
      ) : null}
    </section>
  );
}
