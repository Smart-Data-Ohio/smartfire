import type { ReactNode } from "react";
import "./skeleton.css";

interface SkeletonProps {
  readonly width?: number | string;
  readonly height?: number | string;
  readonly radius?: "sm" | "md" | "pill";
}

/** A placeholder bone that breathes while content loads. Decorative. */
export function Skeleton({ width = "100%", height = 12, radius = "sm" }: SkeletonProps) {
  return (
    <span
      className="skeleton t-skel-bone is-pulsing"
      data-radius={radius}
      style={{ width, height }}
      aria-hidden="true"
    />
  );
}

interface SkeletonRevealProps {
  readonly loading: boolean;
  /** The placeholder layout, built from Skeleton bones; it should match the content's size. */
  readonly skeleton: ReactNode;
  readonly children: ReactNode;
}

/**
 * Content that resolves out of its skeleton (the transitions.dev skeleton reveal): both share one
 * grid cell, the skeleton blurs away as the content sharpens in, and nothing shifts.
 */
export function SkeletonReveal({ loading, skeleton, children }: SkeletonRevealProps) {
  return (
    <div className={`t-skel${loading ? "" : " is-revealed"}`} aria-busy={loading}>
      <div className="t-skel-skeleton is-pulsing" aria-hidden="true" inert>
        {skeleton}
      </div>
      <div className="t-skel-content" inert={loading}>
        {children}
      </div>
    </div>
  );
}
