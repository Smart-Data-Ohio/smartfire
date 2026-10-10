import { Schema } from "effect";
import type { AuditLogEntry as GeneratedAuditLogEntry } from "../../gen/AuditLogEntry.ts";
import type { AuditLogFilters as GeneratedAuditLogFilters } from "../../gen/AuditLogFilters.ts";
import type { AuditLogPage as GeneratedAuditLogPage } from "../../gen/AuditLogPage.ts";
import type { CreateIcon as GeneratedCreateIcon } from "../../gen/CreateIcon.ts";
import type { CustomStyles as GeneratedCustomStyles } from "../../gen/CustomStyles.ts";
import type { DeliveryHealth as GeneratedDeliveryHealth } from "../../gen/DeliveryHealth.ts";
import type { EmailHealth as GeneratedEmailHealth } from "../../gen/EmailHealth.ts";
import type { FizzyHealth as GeneratedFizzyHealth } from "../../gen/FizzyHealth.ts";
import type { GithubHealth as GeneratedGithubHealth } from "../../gen/GithubHealth.ts";
import type { GoogleHealth as GeneratedGoogleHealth } from "../../gen/GoogleHealth.ts";
import type { HealthIssue as GeneratedHealthIssue } from "../../gen/HealthIssue.ts";
import type { IntegrationsHealth as GeneratedIntegrationsHealth } from "../../gen/IntegrationsHealth.ts";
import type { PeoplePage as GeneratedPeoplePage } from "../../gen/PeoplePage.ts";
import type { Person as GeneratedPerson } from "../../gen/Person.ts";
import type { PersonChange as GeneratedPersonChange } from "../../gen/PersonChange.ts";
import type { PersonRemoved as GeneratedPersonRemoved } from "../../gen/PersonRemoved.ts";
import type { PersonRole as GeneratedPersonRole } from "../../gen/PersonRole.ts";
import type { PushChannelExpiry as GeneratedPushChannelExpiry } from "../../gen/PushChannelExpiry.ts";
import type { UpdateBanner as GeneratedUpdateBanner } from "../../gen/UpdateBanner.ts";
import type { UpdateLogo as GeneratedUpdateLogo } from "../../gen/UpdateLogo.ts";
import type { UpdatePerson as GeneratedUpdatePerson } from "../../gen/UpdatePerson.ts";
import type { UpdateWorkspace as GeneratedUpdateWorkspace } from "../../gen/UpdateWorkspace.ts";
import type { Workspace as GeneratedWorkspace } from "../../gen/Workspace.ts";
import type { WorkspaceBranding as GeneratedWorkspaceBranding } from "../../gen/WorkspaceBranding.ts";
import type { WorkspaceIcon as GeneratedWorkspaceIcon } from "../../gen/WorkspaceIcon.ts";
import type { WorkspaceIconList as GeneratedWorkspaceIconList } from "../../gen/WorkspaceIconList.ts";
import { AuditLogEntryId, UserId, WorkspaceIconId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** The workspace: name, logo, join link and the room-creation switch. */
export const Workspace = Schema.Struct({
  name: Schema.String,
  logoUrl: Schema.String,
  logoAttached: Schema.Boolean,
  /** An animated logo's first frame; `null` when the logo isn't animated. */
  logoStillUrl: Schema.NullOr(Schema.String),
  /** The banner behind the sidebar's workspace name; `null` when none is uploaded. */
  bannerUrl: Schema.NullOr(Schema.String),
  /** An animated banner's first frame; `null` when the banner isn't animated. */
  bannerStillUrl: Schema.NullOr(Schema.String),
  joinUrl: Schema.String,
  canAdminister: Schema.Boolean,
  restrictRoomCreationToAdministrators: Schema.Boolean,
  uploadLimitBytes: Schema.Int,
  version: Schema.String,
});

export type Workspace = typeof Workspace.Type;

export type WorkspacePin = Assert<Pinned<typeof Workspace, GeneratedWorkspace>>;

/** `PATCH /admin/workspace`: `null` leaves a key as it is. */
export const UpdateWorkspace = Schema.Struct({
  name: Schema.NullOr(Schema.String),
  restrictRoomCreationToAdministrators: Schema.NullOr(Schema.Boolean),
  uploadLimitBytes: Schema.NullOr(Schema.Int),
});

export type UpdateWorkspace = typeof UpdateWorkspace.Type;

export type UpdateWorkspacePin = Assert<Pinned<typeof UpdateWorkspace, GeneratedUpdateWorkspace>>;

/** `PUT /admin/workspace/logo`. */
export const UpdateLogo = Schema.Struct({ signedId: Schema.String });

export type UpdateLogoPin = Assert<Pinned<typeof UpdateLogo, GeneratedUpdateLogo>>;

/** `PUT /admin/workspace/banner`. */
export const UpdateBanner = Schema.Struct({ signedId: Schema.String });

export type UpdateBannerPin = Assert<Pinned<typeof UpdateBanner, GeneratedUpdateBanner>>;

/**
 * `workspace.updated`, on everyone's `user` topic: the workspace's name, logo or banner changed.
 * A `null` image means none is uploaded; a still is set only for an animated image.
 */
export const WorkspaceBranding = Schema.Struct({
  name: Schema.String,
  logoUrl: Schema.NullOr(Schema.String),
  logoStillUrl: Schema.NullOr(Schema.String),
  bannerUrl: Schema.NullOr(Schema.String),
  bannerStillUrl: Schema.NullOr(Schema.String),
});

export type WorkspaceBranding = typeof WorkspaceBranding.Type;

export type WorkspaceBrandingPin = Assert<
  Pinned<typeof WorkspaceBranding, GeneratedWorkspaceBranding>
>;

/** A role an administrator can give someone. */
export const PersonRole = Schema.Literals(["member", "administrator"]);

export type PersonRole = typeof PersonRole.Type;

export type PersonRolePin = Assert<Pinned<typeof PersonRole, GeneratedPersonRole>>;

/** One active person on the account page. */
export const Person = Schema.Struct({
  id: UserId,
  name: Schema.String,
  avatarUrl: Schema.String,
  role: PersonRole,
  banned: Schema.Boolean,
  you: Schema.Boolean,
  twoFactorEnabled: Schema.Boolean,
  emailAddress: Schema.NullOr(Schema.String),
  googleIdentityEmail: Schema.NullOr(Schema.String),
  offerGoogleEmailLink: Schema.Boolean,
});

export type Person = typeof Person.Type;

export type PersonPin = Assert<Pinned<typeof Person, GeneratedPerson>>;

/** A page of people, administrators first. */
export const PeoplePage = Schema.Struct({
  people: Schema.Array(Person),
  nextPage: Schema.NullOr(Schema.String),
});

export type PeoplePage = typeof PeoplePage.Type;

export type PeoplePagePin = Assert<Pinned<typeof PeoplePage, GeneratedPeoplePage>>;

/** `PATCH /admin/people/:id`. */
export const UpdatePerson = Schema.Struct({ role: PersonRole });

export type UpdatePersonPin = Assert<Pinned<typeof UpdatePerson, GeneratedUpdatePerson>>;

/** One person as a change left them, with the classic notice. */
export const PersonChange = Schema.Struct({ person: Person, notice: Schema.NullOr(Schema.String) });

export type PersonChange = typeof PersonChange.Type;

export type PersonChangePin = Assert<Pinned<typeof PersonChange, GeneratedPersonChange>>;

/** `DELETE /admin/people/:id`. */
export const PersonRemoved = Schema.Struct({ id: UserId });

export type PersonRemoved = typeof PersonRemoved.Type;

export type PersonRemovedPin = Assert<Pinned<typeof PersonRemoved, GeneratedPersonRemoved>>;

/** The account's custom CSS. */
export const CustomStyles = Schema.Struct({ css: Schema.NullOr(Schema.String) });

export type CustomStyles = typeof CustomStyles.Type;

export type CustomStylesPin = Assert<Pinned<typeof CustomStyles, GeneratedCustomStyles>>;

/** A workspace icon members use as a `:shortcode:`. */
export const WorkspaceIcon = Schema.Struct({
  id: WorkspaceIconId,
  name: Schema.String,
  title: Schema.String,
  creatorName: Schema.String,
  imageUrl: Schema.String,
});

export type WorkspaceIcon = typeof WorkspaceIcon.Type;

export type WorkspaceIconPin = Assert<Pinned<typeof WorkspaceIcon, GeneratedWorkspaceIcon>>;

/** Every workspace icon. */
export const WorkspaceIconList = Schema.Struct({ icons: Schema.Array(WorkspaceIcon) });

export type WorkspaceIconList = typeof WorkspaceIconList.Type;

export type WorkspaceIconListPin = Assert<
  Pinned<typeof WorkspaceIconList, GeneratedWorkspaceIconList>
>;

/** `POST /admin/icons`. */
export const CreateIcon = Schema.Struct({
  name: Schema.String,
  title: Schema.String,
  signedId: Schema.NullOr(Schema.String),
});

export type CreateIconPin = Assert<Pinned<typeof CreateIcon, GeneratedCreateIcon>>;

/** The audit log's filters as the server read them. */
export const AuditLogFilters = Schema.Struct({
  actor: Schema.NullOr(Schema.String),
  action: Schema.NullOr(Schema.String),
  targetType: Schema.NullOr(Schema.String),
  from: Schema.NullOr(Schema.String),
  to: Schema.NullOr(Schema.String),
});

export type AuditLogFilters = typeof AuditLogFilters.Type;

export type AuditLogFiltersPin = Assert<Pinned<typeof AuditLogFilters, GeneratedAuditLogFilters>>;

/** One audit log row. */
export const AuditLogEntry = Schema.Struct({
  id: AuditLogEntryId,
  createdAt: Timestamp,
  action: Schema.String,
  actor: Schema.NullOr(Schema.String),
  target: Schema.NullOr(Schema.String),
  targetType: Schema.NullOr(Schema.String),
  changes: Schema.String,
  ipAddress: Schema.NullOr(Schema.String),
});

export type AuditLogEntry = typeof AuditLogEntry.Type;

export type AuditLogEntryPin = Assert<Pinned<typeof AuditLogEntry, GeneratedAuditLogEntry>>;

/** A page of the audit log, newest first. */
export const AuditLogPage = Schema.Struct({
  filters: AuditLogFilters,
  entries: Schema.Array(AuditLogEntry),
  nextPage: Schema.NullOr(Schema.String),
  actions: Schema.Array(Schema.String),
  targetTypes: Schema.Array(Schema.String),
  exportUrl: Schema.String,
  exportTruncated: Schema.Boolean,
  exportLimit: Schema.Int,
  timeZone: Schema.String,
});

export type AuditLogPage = typeof AuditLogPage.Type;

export type AuditLogPagePin = Assert<Pinned<typeof AuditLogPage, GeneratedAuditLogPage>>;

/** One problem a health section lists. */
export const HealthIssue = Schema.Struct({ subject: Schema.String, detail: Schema.String });

export type HealthIssue = typeof HealthIssue.Type;

export type HealthIssuePin = Assert<Pinned<typeof HealthIssue, GeneratedHealthIssue>>;

/** GitHub's health. */
export const GithubHealth = Schema.Struct({
  workspaceToken: Schema.Boolean,
  appConfigured: Schema.Boolean,
  webhookSecret: Schema.Boolean,
  connected: Schema.Int,
  appTokens: Schema.Int,
  deliveries24h: Schema.Int,
  disconnected: Schema.Array(HealthIssue),
  lastErrors: Schema.Array(HealthIssue),
  fetchErrors: Schema.Array(HealthIssue),
});

export type GithubHealthPin = Assert<Pinned<typeof GithubHealth, GeneratedGithubHealth>>;

/** A calendar push channel close to expiring. */
export const PushChannelExpiry = Schema.Struct({
  userId: UserId,
  expiresAt: Schema.NullOr(Timestamp),
  error: Schema.NullOr(Schema.String),
});

export type PushChannelExpiryPin = Assert<
  Pinned<typeof PushChannelExpiry, GeneratedPushChannelExpiry>
>;

/** Google Calendar's health. */
export const GoogleHealth = Schema.Struct({
  configured: Schema.Boolean,
  connected: Schema.Int,
  pushEnabled: Schema.Boolean,
  pushChannels: Schema.Int,
  disconnected: Schema.Array(HealthIssue),
  entryErrors: Schema.Array(HealthIssue),
  expiring: Schema.Array(PushChannelExpiry),
});

export type GoogleHealthPin = Assert<Pinned<typeof GoogleHealth, GeneratedGoogleHealth>>;

/** Fizzy's health. */
export const FizzyHealth = Schema.Struct({
  configured: Schema.Boolean,
  note: Schema.NullOr(Schema.String),
});

export type FizzyHealthPin = Assert<Pinned<typeof FizzyHealth, GeneratedFizzyHealth>>;

/** Agent webhook delivery. */
export const DeliveryHealth = Schema.Struct({
  pending: Schema.Int,
  failed24h: Schema.Int,
  recentErrors: Schema.Array(HealthIssue),
});

export type DeliveryHealthPin = Assert<Pinned<typeof DeliveryHealth, GeneratedDeliveryHealth>>;

/** Email to room. */
export const EmailHealth = Schema.Struct({
  enabled: Schema.Boolean,
  roomsWithAddresses: Schema.Int,
});

export type EmailHealthPin = Assert<Pinned<typeof EmailHealth, GeneratedEmailHealth>>;

/** `GET /admin/integrations_health`. */
export const IntegrationsHealth = Schema.Struct({
  github: GithubHealth,
  google: GoogleHealth,
  fizzy: FizzyHealth,
  agentDelivery: DeliveryHealth,
  email: EmailHealth,
});

export type IntegrationsHealth = typeof IntegrationsHealth.Type;

export type IntegrationsHealthPin = Assert<
  Pinned<typeof IntegrationsHealth, GeneratedIntegrationsHealth>
>;
