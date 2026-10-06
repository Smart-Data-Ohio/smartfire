import { useRef } from "react";
import { UserAvatar } from "../people/user-avatar.tsx";

/** How far a hovered face lifts and how much of that its neighbours share, per step away. */
function tunable(name: string, fallback: number): number {
  const value = Number.parseFloat(
    getComputedStyle(document.documentElement).getPropertyValue(name),
  );

  return Number.isFinite(value) ? value : fallback;
}

function easing(name: string, fallback: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

interface AvatarGroupProps {
  readonly userIds: readonly number[];
  readonly size?: number;
  /** Faces past this many are left out. */
  readonly max?: number;
}

/**
 * Overlapping faces (thread repliers, the header's member stack) with the transitions.dev
 * avatar-group-hover comb: the face under the pointer lifts, its neighbours follow with a
 * falloff, and all of them spring back when the pointer leaves. Decorative; the control around
 * it names who's there.
 */
export function AvatarGroup({ userIds, size = 20, max = 3 }: AvatarGroupProps) {
  const rootRef = useRef<HTMLSpanElement | null>(null);

  const setShifts = (active: number | null) => {
    const root = rootRef.current;

    if (root === null) {
      return;
    }

    const lift = tunable("--avatar-lift", -3);
    const falloff = tunable("--avatar-falloff", 0.45);
    const scale = tunable("--avatar-scale", 1.08);

    const curve =
      active === null
        ? easing("--avatar-ease-out", "cubic-bezier(0.34, 3.85, 0.64, 1)")
        : easing("--avatar-ease-in", "cubic-bezier(0.22, 1, 0.36, 1)");

    root.querySelectorAll<HTMLElement>(".t-avatar").forEach((face, index) => {
      // The curve goes first: a transition takes the timing function current when it starts.
      face.style.transitionTimingFunction = curve;

      if (active === null) {
        face.style.setProperty("--shift", "0px");
        face.style.setProperty("--scale-active", "1");

        return;
      }

      const distance = Math.abs(index - active);

      face.style.setProperty("--shift", `${(lift * falloff ** distance).toFixed(3)}px`);
      face.style.setProperty("--scale-active", index === active ? String(scale) : "1");
    });
  };

  return (
    <span
      ref={rootRef}
      className="avatar-group t-avatar-group"
      style={{ "--avatar-group-size": `${size}px` }}
      aria-hidden="true"
      onPointerLeave={() => setShifts(null)}
    >
      {userIds.slice(0, max).map((id, index) => (
        <span key={id} className="t-avatar" onPointerEnter={() => setShifts(index)}>
          <UserAvatar userId={id} size={size} decorative />
        </span>
      ))}
    </span>
  );
}
