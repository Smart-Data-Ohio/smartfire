import type { TwoFactorRequirement } from "../gen/TwoFactorRequirement.ts";

/** Hears every enrollment requirement an API request answered with. */
export type TwoFactorRequirementListener = (requirement: TwoFactorRequirement) => void;

const listeners = new Set<TwoFactorRequirementListener>();

/**
 * Listens for the enrollment gate: a request refused because the session must set up (or verify)
 * two-step sign-in first. The signed-in shell routes to the setup screen. Returns the unsubscribe.
 */
export function onTwoFactorRequirement(listener: TwoFactorRequirementListener): () => void {
  listeners.add(listener);

  return () => {
    listeners.delete(listener);
  };
}

/** The API client's report of a `TwoFactorRequired` refusal, to whoever listens. */
export function reportTwoFactorRequirement(requirement: TwoFactorRequirement): void {
  for (const listener of [...listeners]) {
    listener(requirement);
  }
}
