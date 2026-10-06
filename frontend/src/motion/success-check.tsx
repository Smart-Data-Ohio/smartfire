interface SuccessCheckProps {
  /** Mount it, or flip this to true, and the check draws itself in. */
  readonly show?: boolean;
  readonly size?: number;
  readonly className?: string;
}

/** The transitions.dev "success check": a check that fades, settles and draws in. Decorative. */
export function SuccessCheck({ show = true, size = 16, className }: SuccessCheckProps) {
  return (
    <span
      className={className === undefined ? "t-success-check" : `t-success-check ${className}`}
      data-state={show ? "in" : "out"}
      aria-hidden="true"
    >
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth={2.25}
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <path d="M5 12.5l4.5 4.5L19 7.5" pathLength={22} />
      </svg>
    </span>
  );
}
