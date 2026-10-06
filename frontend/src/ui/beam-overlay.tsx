import { BorderBeam } from "border-beam";

interface BeamOverlayProps {
  readonly radius: number;
  readonly theme: "light" | "dark";
}

/** The lazily loaded half of Beam: border-beam, laid over the host surface. */
export default function BeamOverlay({ radius, theme }: BeamOverlayProps) {
  return (
    <span className="beam-overlay" aria-hidden="true">
      <BorderBeam
        className="beam-overlay-effect"
        size="md"
        colorVariant="ocean"
        hueRange={24}
        strength={0.8}
        theme={theme}
        borderRadius={radius}
      >
        <span className="beam-fill" />
      </BorderBeam>
    </span>
  );
}
