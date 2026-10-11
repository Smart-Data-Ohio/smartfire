import { useRouter } from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import { twoFactorSetup } from "../../sync/two-factor-setup.ts";
import { safeReturnPath } from "./two-factor-search.ts";

/**
 * The enrollment gate, in the shell: when any request answers that this session must set up
 * two-step sign-in first, the SPA's setup screen replaces the page, and returns here once it's
 * set up. The sign-in challenge requirement is left alone: that session has ended, and the
 * client already sends it to sign in.
 *
 * The shell's requests are refused together, so only the first refusal routes: the others would
 * each route again, from the setup screen to itself.
 */
export function useTwoFactorSetupGate(): void {
  const router = useRouter();
  const routed = useRef(false);

  useEffect(
    () =>
      twoFactorSetup.onRequired((requirement) => {
        if (requirement.kind !== "setup" || routed.current) return;

        const here = router.history.location;

        if (here.pathname.endsWith("/two_factor/setup")) return;

        routed.current = true;

        const returnTo = safeReturnPath(`${here.pathname}${here.search}`);

        void router.navigate({
          to: "/two_factor/setup",
          search: returnTo === null ? { required: true } : { return_to: returnTo, required: true },
          replace: true,
        });
      }),
    [router],
  );
}
