//! Workspace administration (S7): `/api/v1/admin/*`, the SPA's twin of the classic account pages
//! (`accounts#edit` and the people list on it, `accounts/custom_styles`, `accounts/icons`,
//! `accounts/audit_logs` and `accounts/integrations_health`).
//!
//! Everyone can read the workspace and its people, as everyone can open the classic account page;
//! every write and the other pages need an administrator (`Forbidden` otherwise). The writes the
//! classic pages guard with a password confirmation (role changes, removal, custom styles and a
//! new join link) answer `ApiError::SudoRequired` when it has lapsed: the client opens `/sudo/new`,
//! which comes back to the page that asked once the person confirms. A rejected change is
//! `ApiError::Validation` with the classic page's messages, its `fields` keyed by the wire names.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Timestamp;

/// `GET /api/v1/admin/workspace`, and the answer to every workspace write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Workspace {
    pub name: String,
    /// The logo image (the stock app icon when none is attached), with a cache version.
    pub logo_url: String,
    pub logo_still_url: Option<String>,
    pub banner_url: Option<String>,
    pub banner_still_url: Option<String>,
    /// An uploaded logo is attached (it can be removed).
    pub logo_attached: bool,
    /// The full join link everyone may share (`/join/:join_code`).
    pub join_url: String,
    /// The signed-in person is an administrator: they see the controls.
    pub can_administer: bool,
    /// "Must be admin to create new rooms".
    pub restrict_room_creation_to_administrators: bool,
    /// Maximum bytes per uploaded file. Defaults to 100 MiB.
    pub upload_limit_bytes: i64,
    /// The footer's "Smartfire version" badge text.
    pub version: String,
    /// Plain-text workspace description, empty when unset.
    pub description: String,
    /// The optional single-instance invite alias.
    pub vanity_slug: Option<String>,
    pub vanity_url: Option<String>,
}

/// `PATCH /api/v1/admin/workspace` (`accounts#update`). `null` leaves a key as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateWorkspace {
    pub name: Option<String>,
    pub restrict_room_creation_to_administrators: Option<bool>,
    pub upload_limit_bytes: Option<i64>,
    /// An empty string clears the description; null leaves it unchanged.
    pub description: Option<String>,
    /// An empty string removes the alias; null leaves it unchanged.
    pub vanity_slug: Option<String>,
}

/// `PUT /api/v1/admin/workspace/logo`: a blob uploaded with `POST /api/v1/uploads` becomes the
/// logo. `DELETE` removes it. `POST /api/v1/admin/workspace/join_code` makes a new join link
/// (the old one stops working). Each answers the [`Workspace`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateLogo {
    pub signed_id: String,
}

/// `PUT /api/v1/admin/workspace/banner`: attach an uploaded image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateBanner {
    pub signed_id: String,
}

/// A role an administrator can give someone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum PersonRole {
    Member,
    Administrator,
}

/// One active person on the account page (bots are under bots).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub avatar_url: String,
    pub role: PersonRole,
    /// Banned people stay listed, struck through.
    pub banned: bool,
    /// The signed-in person (no role, removal or reset controls on their own row).
    pub you: bool,
    /// The fields below are for administrators: `false` or `null` for everyone else.
    pub two_factor_enabled: bool,
    pub email_address: Option<String>,
    /// The Google account linked for sign-in; an administrator can unlink it.
    pub google_identity_email: Option<String>,
    /// The person chose their email themselves, so Google sign-in waits for an administrator to
    /// allow it (`POST .../google_link`).
    pub offer_google_email_link: bool,
}

/// `GET /api/v1/admin/people?page=`: active people, administrators first on the first page, 500
/// a page as the classic list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PeoplePage {
    pub people: Vec<Person>,
    /// Pass as `page` for the rest; `null` on the last page.
    pub next_page: Option<String>,
}

/// `PATCH /api/v1/admin/people/:id` (`accounts/users#update`). Needs sudo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdatePerson {
    pub role: PersonRole,
}

/// The answer to a change to one person: the row as it is now, and the classic page's notice.
/// `POST /api/v1/admin/people/:id/two_factor_reset` refuses yourself and people without
/// two-step sign-in with `Validation` (no `fields`), carrying the classic alert.
/// `POST`/`DELETE /api/v1/admin/people/:id/google_link` allow or unlink Google sign-in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonChange {
    pub person: Person,
    pub notice: Option<String>,
}

/// `DELETE /api/v1/admin/people/:id` (`accounts/users#destroy`): deactivated. Needs sudo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PersonRemoved {
    pub id: i64,
}

/// `GET`/`PATCH /api/v1/admin/custom_styles`: the account's custom CSS for the classic pages
/// (`null` when none). The write sends the whole text (blank or `null` clears it; a body without
/// `css` changes nothing) and needs sudo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CustomStyles {
    pub css: Option<String>,
}

/// One workspace icon members can use as a `:shortcode:`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkspaceIcon {
    pub id: i64,
    /// The shortcode without colons.
    pub name: String,
    pub title: String,
    pub creator_name: String,
    /// `/icons/:name`.
    pub image_url: String,
    pub animated: bool,
    /// First-frame PNG for animated emoji, otherwise the original image URL.
    pub still_url: String,
}

/// `GET /api/v1/admin/icons`, and the answer to adding (`POST`) or deleting
/// (`DELETE /api/v1/admin/icons/:id`) one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkspaceIconList {
    pub icons: Vec<WorkspaceIcon>,
    /// Static emoji have unlimited capacity. This limit applies only to animated emoji.
    pub animated_limit: i64,
    pub animated_usage: i64,
}

/// `POST /api/v1/admin/icons` (`accounts/icons#create`). A refusal is `Validation` with `fields`
/// among `name`, `title` and `image`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CreateIcon {
    pub name: String,
    pub title: String,
    /// The uploaded SVG, PNG, GIF or WebP; `null` is refused as a missing image.
    pub signed_id: Option<String>,
}

/// The audit log's filters as the server read them (unknown actions and types and unreadable
/// dates are dropped, as on the classic page). Query keys: `actor`, `action`, `targetType`,
/// `from`, `to` (dates, `YYYY-MM-DD`) and `page`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AuditLogFilters {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// One audit log row. Labels are `null` when the record had none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AuditLogEntry {
    pub id: i64,
    pub created_at: Timestamp,
    pub action: String,
    pub actor: Option<String>,
    pub target: Option<String>,
    pub target_type: Option<String>,
    /// The classic page's one-line summary of what changed (blank when nothing did).
    pub changes: String,
    pub ip_address: Option<String>,
}

/// `GET /api/v1/admin/audit_log`: newest first, 50 a page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AuditLogPage {
    pub filters: AuditLogFilters,
    pub entries: Vec<AuditLogEntry>,
    /// Pass as `page` for older entries; `null` on the last page.
    pub next_page: Option<String>,
    /// The choices for the action and type filters.
    pub actions: Vec<String>,
    pub target_types: Vec<String>,
    /// The classic CSV download for these filters (it asks for a password confirmation).
    pub export_url: String,
    /// The export stops at `exportLimit` rows and these filters match more.
    pub export_truncated: bool,
    pub export_limit: i64,
    /// The IANA zone the `from` and `to` dates are read in (the viewer's profile zone, UTC when
    /// unset): show the entries' times in it too.
    pub time_zone: String,
}

/// One problem a health section lists: what it concerns, and what went wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HealthIssue {
    /// As the classic page labels it: a login, `event 4 / user 7`, `owner/repo#12`, ...
    pub subject: String,
    pub detail: String,
}

/// GitHub: workspace token, App, webhook secret, accounts and recent failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GithubHealth {
    pub workspace_token: bool,
    pub app_configured: bool,
    pub webhook_secret: bool,
    pub connected: i64,
    /// Of `connected`, those through the GitHub App (the rest use a personal token).
    pub app_tokens: i64,
    pub deliveries_24h: i64,
    pub disconnected: Vec<HealthIssue>,
    pub last_errors: Vec<HealthIssue>,
    pub fetch_errors: Vec<HealthIssue>,
}

/// A calendar push channel that expires within a day (or has no expiry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PushChannelExpiry {
    pub user_id: i64,
    pub expires_at: Option<Timestamp>,
    pub error: Option<String>,
}

/// Google Calendar: OAuth client, accounts, push channels and entry errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GoogleHealth {
    pub configured: bool,
    pub connected: i64,
    pub push_enabled: bool,
    pub push_channels: i64,
    pub disconnected: Vec<HealthIssue>,
    pub entry_errors: Vec<HealthIssue>,
    pub expiring: Vec<PushChannelExpiry>,
}

/// Fizzy: configured, or the note saying why not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FizzyHealth {
    pub configured: bool,
    pub note: Option<String>,
}

/// Agent webhook delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeliveryHealth {
    pub pending: i64,
    pub failed_24h: i64,
    pub recent_errors: Vec<HealthIssue>,
}

/// Email to room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EmailHealth {
    pub enabled: bool,
    pub rooms_with_addresses: i64,
}

/// `GET /api/v1/admin/integrations_health`: read only, never cached. No credential reaches it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IntegrationsHealth {
    pub github: GithubHealth,
    pub google: GoogleHealth,
    pub fizzy: FizzyHealth,
    pub agent_delivery: DeliveryHealth,
    pub email: EmailHealth,
}

/// A room the welcome card suggests, as administrators set it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WelcomeRoom {
    pub room_id: i64,
    /// One Unicode emoji shown on the card; `null` for none.
    pub emoji: Option<String>,
    /// Up to 100 characters of plain text.
    pub description: String,
}

/// One server guide entry as administrators set it: a link to a room (`roomId`) or a short
/// markdown note (`body`), never both.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GuideResource {
    pub title: String,
    pub emoji: Option<String>,
    pub room_id: Option<i64>,
    pub body: Option<String>,
}

/// The welcome screen and server guide as stored in `accounts.settings` under `welcome`. Never
/// sent as is: administrators get [`WelcomeAdmin`], everyone else [`WorkspaceWelcome`]. An empty
/// message with no rooms turns the welcome card off; an empty guide hides the guide page.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WelcomeSettings {
    /// Short markdown, up to 1,000 characters.
    pub message: String,
    /// Up to 5 suggested rooms, in order.
    pub rooms: Vec<WelcomeRoom>,
    /// Up to 10 resources, in order.
    pub guide: Vec<GuideResource>,
}

/// A suggested room in the editor. `roomId` names a room this administrator can see. A saved room
/// they can't see comes with `roomId: null` and an opaque `keep` token instead, so neither its id
/// nor its name reaches them; sending the token back keeps that room in the row. Exactly one of
/// the two is set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WelcomeRoomEdit {
    pub room_id: Option<i64>,
    #[serde(default)]
    pub keep: Option<String>,
    /// One Unicode emoji shown on the card; `null` for none.
    pub emoji: Option<String>,
    /// Up to 100 characters of plain text.
    pub description: String,
}

/// A guide entry in the editor: a room link (`roomId`, or `keep` for a saved room this
/// administrator can't see, as in [`WelcomeRoomEdit`]) or a short markdown note (`body`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GuideResourceEdit {
    pub title: String,
    pub emoji: Option<String>,
    pub room_id: Option<i64>,
    #[serde(default)]
    pub keep: Option<String>,
    pub body: Option<String>,
}

/// `PATCH /api/v1/admin/welcome`: replaces the sections it names and leaves the rest as stored,
/// so the welcome and the guide save on their own. A refusal is `Validation` with `fields` among
/// `message`, `rooms` and `guide`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WelcomeUpdate {
    /// Short markdown, up to 1,000 characters.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Up to 5 suggested rooms, in order.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rooms: Option<Vec<WelcomeRoomEdit>>,
    /// Up to 10 resources, in order.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guide: Option<Vec<GuideResourceEdit>>,
}

/// A room an administrator may suggest or link to: one they belong to, or an open room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WelcomeRoomChoice {
    pub id: i64,
    pub name: String,
}

/// `GET /api/v1/admin/welcome` (administrators only), and the answer to the `PATCH`: the
/// configuration as this administrator may see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WelcomeAdmin {
    pub message: String,
    pub rooms: Vec<WelcomeRoomEdit>,
    pub guide: Vec<GuideResourceEdit>,
    /// The rooms the editor offers, by name.
    pub choices: Vec<WelcomeRoomChoice>,
}

/// A suggested room as one viewer sees it: only rooms they belong to or may join are sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WelcomeRoomCard {
    pub room_id: i64,
    pub name: String,
    pub emoji: Option<String>,
    pub description: String,
    /// The viewer belongs to the room (open it); otherwise it's an open room they may join.
    pub joined: bool,
}

/// A guide entry as one viewer sees it. A room link the viewer can't open or join is left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GuideEntry {
    pub title: String,
    pub emoji: Option<String>,
    /// The note, rendered and sanitized as a message's markdown is; `null` for a room link.
    pub body_html: Option<String>,
    /// The linked room, for a room link.
    pub room: Option<GuideRoom>,
}

/// The room a guide entry links to, as one viewer sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GuideRoom {
    pub id: i64,
    pub name: String,
    /// The viewer belongs to the room; otherwise it's an open room they may join.
    pub joined: bool,
}

/// `GET /api/v1/welcome`: the welcome card and server guide for the signed-in person. Nothing
/// configured is an empty message, no rooms and no guide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkspaceWelcome {
    pub name: String,
    /// The workspace description (plain text), empty when unset.
    pub description: String,
    /// The welcome message, rendered and sanitized; `null` when there is none.
    pub message_html: Option<String>,
    pub rooms: Vec<WelcomeRoomCard>,
    pub guide: Vec<GuideEntry>,
    /// The viewer may edit these in the workspace settings.
    pub can_administer: bool,
}
