import { useEffect, useState } from "react";
import { inlineMentions } from "../../../lib/body-html.ts";
import { usePresence } from "../../../motion/presence.ts";
import { composerActions } from "../../../sync/composer-actions.ts";
import { IconButton } from "../../../ui/icon-button.tsx";
import { Icon } from "../../../ui/icons/icon.tsx";
import { Skeleton } from "../../../ui/skeleton.tsx";
import "./preview.css";

/** Re-render this long after the last keystroke while the preview is open. */
const PREVIEW_DEBOUNCE_MS = 250;

type PreviewState =
  | { readonly status: "loading"; readonly html: string | null }
  | { readonly status: "ready"; readonly html: string }
  | { readonly status: "error"; readonly message: string };

interface PreviewPanelProps {
  readonly open: boolean;
  readonly roomId: number;
  readonly markdown: string;
  readonly onClose: () => void;
}

/**
 * The draft as it will post, rendered by the server (`POST …/messages/preview`, mentions resolved
 * against the room) above the input. It follows the text while open, keeping the last rendering
 * up while the next one loads.
 */
export function PreviewPanel({ open, roomId, markdown, onClose }: PreviewPanelProps) {
  const presence = usePresence<HTMLElement>(open);
  const [state, setState] = useState<PreviewState>({ status: "loading", html: null });
  const source = markdown.trim();

  useEffect(() => {
    if (!open || source === "") {
      return;
    }

    let live = true;

    setState((current) => ({
      status: "loading",
      html: current.status === "error" ? null : current.html,
    }));

    const timer = window.setTimeout(() => {
      composerActions.preview(roomId, source).then(
        (html) => {
          if (live) {
            setState({ status: "ready", html });
          }
        },
        (error: Error) => {
          if (live) {
            setState({ status: "error", message: error.message });
          }
        },
      );
    }, PREVIEW_DEBOUNCE_MS);

    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [open, roomId, source]);

  if (!presence.mounted) {
    return null;
  }

  return (
    <section
      ref={presence.ref}
      className="composer-preview"
      data-state={presence.state}
      aria-label="Message preview"
      aria-live="polite"
      aria-busy={state.status === "loading" || undefined}
    >
      <div className="composer-preview-inner">
        <header className="composer-preview-head">
          <span className="composer-preview-title">
            <Icon name="eye" size={14} />
            Preview
          </span>
          <IconButton
            icon="x"
            label="Close preview"
            shortcut={["Esc"]}
            size="sm"
            onMouseDown={(event) => event.preventDefault()}
            onClick={onClose}
          />
        </header>
        <PreviewBody source={source} state={state} />
      </div>
    </section>
  );
}

function PreviewBody({ source, state }: { readonly source: string; readonly state: PreviewState }) {
  if (source === "") {
    return <p className="composer-preview-empty">Nothing to preview yet. Start typing.</p>;
  }

  if (state.status === "error") {
    return <p className="composer-preview-error">Couldn't render the preview: {state.message}</p>;
  }

  if (state.html === null) {
    return (
      <div className="composer-preview-loading">
        <Skeleton width="70%" height={12} />
        <Skeleton width="45%" height={12} />
      </div>
    );
  }

  return (
    <div
      className="message-body composer-preview-body"
      data-stale={state.status === "loading" || undefined}
      // biome-ignore lint/security/noDangerouslySetInnerHtml: bodyHtml is the server's sanitizer output (crates/richtext), the HTML the classic views render
      dangerouslySetInnerHTML={{ __html: inlineMentions(state.html) }}
    />
  );
}
