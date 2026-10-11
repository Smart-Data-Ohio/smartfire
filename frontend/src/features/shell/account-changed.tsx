import { useId } from "react";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { AuthHeader, AuthScreen } from "../auth/auth-parts.tsx";

/**
 * In place of the whole shell once the server answered this page as another account: nothing of
 * the old one stays on screen or keeps working, and the only way on is the reload.
 */
export function AccountChangedNotice() {
  const titleId = useId();

  return (
    <AuthScreen labelledBy={titleId}>
      <AuthHeader titleId={titleId} title="You signed in as a different account in another tab" />
      <Button
        variant="primary"
        size="lg"
        className="auth-view-wide"
        onClick={() => actions.reloadForNewSignIn()}
      >
        Reload
      </Button>
    </AuthScreen>
  );
}
