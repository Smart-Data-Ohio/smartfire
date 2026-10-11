import { useRouter } from "@tanstack/react-router";
import { useEffect } from "react";
import { twoFactorSetup } from "../../sync/two-factor-setup.ts";
import { safeReturnPath } from "./two-factor-search.ts";

/**
 * The enrollment gate, in the shell: when any request answers that this session must set up
 * two-step sign-in first, the SPA's setup screen replaces the page, and returns here once it's
 * set up. The sign-in challenge requirement is left alone: that session has ended, and the
 * client already sends it to sign in.
 */
export function useTwoFactorSetupGate(): void {
  const router = useRouter();

  useEffect(
    () =>
      twoFactorSetup.onRequired((requirement) => {
        if (requirement.kind !== "setup") return;

        const here = router.history.location;
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
