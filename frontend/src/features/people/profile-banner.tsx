import { type ReactNode, useState } from "react";
import { avatarHue } from "../../ui/avatar-palette.ts";
import { useShownSrc } from "../admin/profile-slot.tsx";
import "./profile-banner.css";

interface ProfileBannerProps {
  readonly userId: number;
  readonly name: string;
  readonly url: string | null;
  readonly stillUrl: string | null;
  /** A file being uploaded in its place, as the settings preview shows it. */
  readonly file?: File | null;
}

/**
 * Someone's profile banner, the strip across the top of their profile: their image (its first
 * frame when motion is reduced), or a wash of their avatar's tint when they have none or it
 * doesn't load.
 */
export function ProfileBanner({ userId, name, url, stillUrl, file = null }: ProfileBannerProps) {
  const src = useShownSrc({ url, stillUrl, file });
  const [failed, setFailed] = useState<string | null>(null);
  const shown = src !== null && src !== failed;

  return (
    <div
      className="profile-banner"
      data-empty={shown ? undefined : true}
      style={{ "--avatar-hue": avatarHue(userId) }}
    >
      {shown ? (
        <img
          className="profile-banner-image"
          src={src}
          alt={`${name}'s profile banner`}
          decoding="async"
          draggable={false}
          onError={() => setFailed(src)}
        />
      ) : null}
    </div>
  );
}

interface ProfileHeroProps {
  readonly userId: number;
  readonly name: string;
  readonly url: string | null;
  readonly stillUrl: string | null;
  readonly file?: File | null;
  /** `card` for the narrow details pane and the settings preview. */
  readonly size?: "page" | "card";
  /** The avatar, drawn over the banner's lower edge. */
  readonly children: ReactNode;
}

/** The banner with the avatar overlapping its lower edge, ringed in the pane's colour. */
export function ProfileHero({ size = "page", children, ...banner }: ProfileHeroProps) {
  return (
    <div className="profile-hero" data-size={size}>
      <ProfileBanner {...banner} />
      <div className="profile-hero-avatar">{children}</div>
    </div>
  );
}
