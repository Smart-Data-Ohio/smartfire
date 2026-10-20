import { useState } from "react";
import type { Workspace } from "../../gen/Workspace.ts";
import { admin } from "../../sync/admin.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { useAdmin } from "./admin-parts.tsx";
import { PROFILE_FORMATS, PROFILE_SLOTS, type ProfileImageKind } from "./profile-format.ts";
import {
  type ImageSlot,
  ProfileSlot,
  type Shown,
  useImageSlot,
  useShownSrc,
} from "./profile-slot.tsx";

function serverImage(workspace: Workspace, kind: ProfileImageKind): Shown {
  if (kind === "logo") {
    return workspace.logoAttached
      ? { url: workspace.logoUrl, stillUrl: workspace.logoStillUrl }
      : { url: null, stillUrl: null };
  }

  if (kind === "invite_splash") return { url: workspace.inviteSplashUrl, stillUrl: null };

  return { url: workspace.bannerUrl, stillUrl: workspace.bannerStillUrl };
}

function initialsOf(name: string): string {
  return name
    .trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((word) => [...word][0] ?? "")
    .join("")
    .toUpperCase();
}

/**
 * The profile as members will see it: the banner at the top of the sidebar with the workspace's
 * name over it, and the icon as the rail shows it. Files being uploaded show here at once.
 */
function ProfilePreview({
  name,
  logo,
  banner,
}: {
  readonly name: string;
  readonly logo: Shown;
  readonly banner: Shown;
}) {
  const logoSrc = useShownSrc(logo);
  const shownBanner = useShownSrc(banner);
  // An image that fails to load leaves the plain header, as the sidebar does.
  const [brokenBanner, setBrokenBanner] = useState<string | null>(null);
  const bannerSrc = shownBanner === brokenBanner ? null : shownBanner;

  return (
    <figure className="profile-preview" aria-label="Preview">
      <div className="profile-preview-rail">
        <span className="profile-preview-icon" data-logo={logoSrc !== null || undefined}>
          {logoSrc === null ? (
            initialsOf(name)
          ) : (
            <img src={logoSrc} alt="" width={40} height={40} draggable={false} />
          )}
        </span>
        <span className="profile-preview-dot" aria-hidden="true" />
        <span className="profile-preview-dot" aria-hidden="true" />
      </div>
      <div className="profile-preview-sidebar">
        <div className="profile-preview-banner" data-empty={bannerSrc === null || undefined}>
          {bannerSrc === null ? null : (
            <img
              className="profile-preview-banner-image"
              src={bannerSrc}
              alt=""
              draggable={false}
              onError={() => setBrokenBanner(bannerSrc)}
            />
          )}
          <span className="profile-preview-name">
            <span className="profile-preview-name-text">{name}</span>
            <Icon name="chevron-down" size={14} />
          </span>
        </div>
        <span className="profile-preview-lines" aria-hidden="true">
          <span />
          <span />
          <span />
          <span />
        </span>
      </div>
      <figcaption className="visually-hidden">
        {`${name}: ${logoSrc === null ? "initials" : "icon"} in the rail, ${bannerSrc === null ? "no banner" : "banner"} behind the name`}
      </figcaption>
    </figure>
  );
}

function InvitePreview({
  workspace,
  logo,
  splash,
}: {
  readonly workspace: Workspace;
  readonly logo: Shown;
  readonly splash: Shown;
}) {
  const logoSrc = useShownSrc(logo);
  const shown = useShownSrc(splash);
  const [broken, setBroken] = useState<string | null>(null);
  const src = shown === broken ? null : shown;

  return (
    <figure className="profile-invite-preview" aria-label="Invite preview">
      {src === null ? null : (
        <img
          className="profile-invite-background"
          src={src}
          alt=""
          onError={() => setBroken(src)}
        />
      )}
      <div className="profile-invite-card">
        {logoSrc === null ? null : <img src={logoSrc} alt="" width={32} height={32} />}
        <span className="text-title">Join {workspace.name}</span>
        {workspace.description === "" ? null : <p className="text-meta">{workspace.description}</p>}
      </div>
      <figcaption className="visually-hidden">The join and About background</figcaption>
    </figure>
  );
}

/**
 * The workspace profile, Discord's server profile: an icon for the rail and a banner for the top of
 * the sidebar, with a preview of both. Administrators upload, replace and remove them; everyone
 * else sees the preview.
 */
export function WorkspaceProfile() {
  const { workspace, replace } = useAdmin();

  const slots: Record<ProfileImageKind, ImageSlot> = {
    logo: useImageSlot({
      save: admin.setLogo,
      remove: admin.removeLogo,
      onChange: replace,
      savedTitle: `${PROFILE_SLOTS.logo.title} updated`,
    }),
    invite_splash: useImageSlot({
      save: admin.setInviteSplash,
      remove: admin.removeInviteSplash,
      onChange: replace,
      savedTitle: "Invite splash updated",
    }),
    banner: useImageSlot({
      save: admin.setBanner,
      remove: admin.removeBanner,
      onChange: replace,
      savedTitle: `${PROFILE_SLOTS.banner.title} updated`,
    }),
  };

  const shown = (kind: ProfileImageKind): Shown => slots[kind].shown(serverImage(workspace, kind));

  return (
    <div className="profile">
      <ProfilePreview name={workspace.name} logo={shown("logo")} banner={shown("banner")} />
      <InvitePreview workspace={workspace} logo={shown("logo")} splash={shown("invite_splash")} />
      {workspace.canAdminister ? (
        <div className="profile-slots">
          {(["logo", "banner", "invite_splash"] as const).map((kind) => (
            <ProfileSlot
              key={kind}
              thumbnail={kind}
              slot={PROFILE_SLOTS[kind]}
              image={serverImage(workspace, kind)}
              pending={slots[kind].pending}
              error={slots[kind].error}
              onFile={slots[kind].upload}
              onRemove={slots[kind].remove}
            />
          ))}
          <p className="profile-formats text-meta">{PROFILE_FORMATS}</p>
        </div>
      ) : null}
    </div>
  );
}
