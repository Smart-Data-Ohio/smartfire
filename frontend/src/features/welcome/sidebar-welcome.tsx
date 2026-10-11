import { useNavigate } from "@tanstack/react-router";
import { usePhoneLayout } from "../panes/use-right-pane.ts";
import { setDestination } from "../shell/view-store.ts";
import { setWelcomeDismissed } from "./welcome-actions.ts";
import { WelcomeCard } from "./welcome-card.tsx";
import { useWelcomeDismissed } from "./welcome-dismissal.ts";
import { hasWelcomeCard, useWelcome } from "./welcome-store.ts";

/**
 * On phones the conversation list is Home, so the welcome card sits at its top until dismissed.
 * Wide screens show it in the main pane instead (HomeView).
 */
export function SidebarWelcome() {
  const phone = usePhoneLayout();
  const { welcome } = useWelcome();
  const welcomeDismissed = useWelcomeDismissed();

  if (!phone || welcomeDismissed || !hasWelcomeCard(welcome)) {
    return null;
  }

  return <WelcomeCard welcome={welcome} compact />;
}

/**
 * The workspace menu's "Show welcome": brings a dismissed welcome card back and goes Home, where
 * it shows: the Home destination too, since on phones the card tops Home's list, not the DMs
 * one. `null` while there's no card to bring back.
 */
export function useShowWelcome(): (() => void) | null {
  const navigate = useNavigate();
  const { welcome } = useWelcome();
  const welcomeDismissed = useWelcomeDismissed();

  if (!welcomeDismissed || !hasWelcomeCard(welcome)) {
    return null;
  }

  return () => {
    setWelcomeDismissed(false);
    setDestination("home");
    void navigate({ to: "/" });
  };
}
