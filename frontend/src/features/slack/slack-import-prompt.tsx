import { useMatchRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import type { SlackPersonal } from "../../gen/SlackPersonal.ts";
import { useStore } from "../../store/store.ts";
import { slack } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import {
  personalGuideProgress,
  SlackImportFacts,
  SlackImportGuideList,
  shouldOfferSlackPrompt,
} from "./slack-guide.tsx";
import "./slack.css";

const DISMISSED_PREFIX = "smartfire.slack-import-prompt.dismissed.";

function dismissedKey(userId: number): string {
  return `${DISMISSED_PREFIX}${userId}`;
}

/** Whether this user has already dismissed the in-app Slack import prompt on this browser. */
export function isSlackPromptDismissed(userId: number): boolean {
  try {
    return localStorage.getItem(dismissedKey(userId)) === "1";
  } catch {
    return false;
  }
}

/** Remembers that this user dismissed the in-app Slack import prompt. */
export function dismissSlackPrompt(userId: number): void {
  try {
    localStorage.setItem(dismissedKey(userId), "1");
  } catch {
    // Unavailable storage only means the dismissal lasts for this session.
  }
}

/**
 * A gentle, dismissible bar shown when workspace Slack import is ready and the person hasn't
 * imported their own Slack history yet. Offers a plain-English "How it works" walkthrough dialog
 * and a one-click jump straight to `/settings/slack`.
 */
export function SlackImportPrompt() {
  const navigate = useNavigate();
  const matchRoute = useMatchRoute();
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  const onSettings = matchRoute({ to: "/settings", fuzzy: true }) !== false;
  const onAdmin = matchRoute({ to: "/admin", fuzzy: true }) !== false;
  const hiddenByRoute = onSettings || onAdmin;

  const [dismissedFor, setDismissedFor] = useState<number | null>(null);

  const [personal, setPersonal] = useState<{
    readonly userId: number;
    readonly page: SlackPersonal;
  } | null>(null);

  const [guideOpen, setGuideOpen] = useState(false);

  const [wasHidden, setWasHidden] = useState(hiddenByRoute);

  if (wasHidden !== hiddenByRoute) {
    setWasHidden(hiddenByRoute);

    if (hiddenByRoute) {
      setPersonal(null);
      setGuideOpen(false);
    }
  }

  const dismissed =
    viewerId === null || dismissedFor === viewerId || isSlackPromptDismissed(viewerId);

  useEffect(() => {
    if (viewerId === null || dismissed || hiddenByRoute) {
      return;
    }

    let cancelled = false;

    void slack.personal().then(
      (page) => {
        if (!cancelled) {
          setPersonal({ userId: viewerId, page });
        }
      },
      () => {
        // Offline or unavailable: stay quiet so the shell never shows an error for a nudge.
      },
    );

    return () => {
      cancelled = true;
    };
  }, [viewerId, dismissed, hiddenByRoute]);

  const page = personal !== null && personal.userId === viewerId ? personal.page : null;

  if (
    viewerId === null ||
    dismissed ||
    hiddenByRoute ||
    page === null ||
    !shouldOfferSlackPrompt(page)
  ) {
    return null;
  }

  const progress = personalGuideProgress(page);

  const goToImport = () => {
    setGuideOpen(false);
    void navigate({ to: "/settings/slack" });
  };

  const dismiss = () => {
    dismissSlackPrompt(viewerId);
    setDismissedFor(viewerId);
    setGuideOpen(false);
  };

  return (
    <>
      <section className="slack-prompt" aria-label="Import your Slack history">
        <div className="slack-prompt-inner">
          <Icon name="download" size={16} className="slack-prompt-icon" />
          <p className="slack-prompt-copy">
            <strong>Moving from Slack?</strong> Bring your DMs, notes, and private channels into
            Smartfire.
          </p>
          <div className="slack-prompt-actions">
            <Button size="sm" variant="ghost" onClick={() => setGuideOpen(true)}>
              How it works
            </Button>
            <Button size="sm" variant="primary" onClick={goToImport}>
              Import from Slack
            </Button>
            <IconButton icon="x" size="sm" label="Dismiss Slack import prompt" onClick={dismiss} />
          </div>
        </div>
      </section>
      <Dialog
        open={guideOpen}
        onOpenChange={setGuideOpen}
        size="md"
        title="Import your personal Slack history"
        description="Bring your direct messages, group DMs, notes to self, and private channels from Slack into Smartfire in a few clicks — no file exports needed."
        footer={
          <>
            <Button variant="secondary" onClick={() => setGuideOpen(false)}>
              Not now
            </Button>
            <Button variant="primary" data-autofocus onClick={goToImport}>
              Go to Slack import
            </Button>
          </>
        }
      >
        <div className="slack-dialog-content">
          <SlackImportGuideList progress={progress} />
          <section className="slack-dialog-facts" aria-label="Good to know">
            <h3 className="slack-dialog-subheading">Good to know</h3>
            <SlackImportFacts />
          </section>
        </div>
      </Dialog>
    </>
  );
}
