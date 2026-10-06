import { lazy, type ReactNode, Suspense } from "react";
import { useResolvedTheme } from "../lib/appearance.ts";
import { useReducedMotion } from "../motion/reduced-motion.ts";
import "./effects.css";

interface SpeakingRingProps {
  /** Voice level, 0 (silence) to 1 (loud). */
  readonly level: number;
  readonly speaking: boolean;
  /** Corner radius of the wrapped avatar or tile, in px. */
  readonly radius?: number;
  readonly children: ReactNode;
}

const VoiceOverlay = lazy(() => import("./speaking-overlay.tsx"));

/**
 * The "who's talking" ring for huddles: a green ring whose strength follows the voice level,
 * with Jakub Antalik's voice-glow layered on when motion is allowed.
 */
export function SpeakingRing({ level, speaking, radius = 8, children }: SpeakingRingProps) {
  const reduced = useReducedMotion();
  const theme = useResolvedTheme();
  const clamped = Math.min(1, Math.max(0, level));

  return (
    <span
      className="speaking-ring"
      data-speaking={speaking || undefined}
      style={{ "--level": clamped, "--ring-radius": `${radius}px` }}
    >
      {children}
      {speaking && !reduced ? (
        <Suspense fallback={null}>
          <VoiceOverlay level={clamped} radius={radius} theme={theme} />
        </Suspense>
      ) : null}
    </span>
  );
}
