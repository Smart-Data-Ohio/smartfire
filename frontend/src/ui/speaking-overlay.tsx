import { VoiceBeam } from "voice-glow";

interface SpeakingOverlayProps {
  readonly level: number;
  readonly radius: number;
  readonly theme: "light" | "dark";
}

/** The lazily loaded half of SpeakingRing: voice-glow around the tile, driven by `level`. */
export default function SpeakingOverlay({ level, radius, theme }: SpeakingOverlayProps) {
  return (
    <span className="speaking-overlay" aria-hidden="true">
      <VoiceBeam
        className="speaking-overlay-effect"
        type="pill"
        level={level}
        colorVariant="forest"
        theme={theme}
        borderRadius={radius}
      >
        <span className="speaking-fill" />
      </VoiceBeam>
    </span>
  );
}
