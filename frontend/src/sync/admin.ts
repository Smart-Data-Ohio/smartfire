/**
 * What the admin screens call: plain promises over the S7 admin endpoints. Failures reject with an
 * `ActionError`: `ConfirmationCancelled` when the person closes the confirmation a guarded write
 * asks for, `SudoRequired` when they leave to confirm with Google and the write can't be kept,
 * `Validation` with the classic page's message (and `fields`) when a change is refused.
 */
import {
  auditLog,
  createIcon,
  createInvite,
  customStyles,
  destroyIcon,
  icons,
  integrationsHealth,
  invites,
  people,
  removeBanner,
  removeLogo,
  removePerson,
  resetJoinCode,
  resetTwoFactor,
  revokeInvite,
  setGoogleLink,
  updateBanner,
  updateCustomStyles,
  updateLogo,
  updatePerson,
  updateWorkspace,
  workspace,
} from "../api/admin-endpoints.ts";
import {
  bot,
  bots as botList,
  connectGithub,
  createBot,
  createGrant,
  credentials,
  disconnectGithub,
  grants,
  issueCredential,
  removeBot,
  resetBotKey,
  resetSigningSecret,
  revokeCredential,
  revokeGrant,
  suspendBot,
  updateBot,
} from "../api/bot-endpoints.ts";
import {
  blocks,
  peopleDirectory,
  personProfile,
  setBanned,
  setIgnored,
} from "../api/people-endpoints.ts";
import { setDndAllowance } from "../api/settings-endpoints.ts";
import {
  cancelSlackRun,
  disconnectSlack,
  personalSlack,
  removeSlackCredentials,
  saveSlackCredentials,
  slackPlan,
  slackRunPage,
  slackRunStatus,
  slackRuns,
  slackSetup,
  startPersonalSlack,
  startSlackCatchUp,
  startSlackDryRun,
  startSlackImport,
  undoSlackRun,
} from "../api/slack-endpoints.ts";
import { adminWelcome, updateWelcome } from "../api/welcome-endpoints.ts";
import type { AuditLogFilters } from "../gen/AuditLogFilters.ts";
import type { AuditLogPage } from "../gen/AuditLogPage.ts";
import type { BlockList } from "../gen/BlockList.ts";
import type { BlockState } from "../gen/BlockState.ts";
import type { Bot } from "../gen/Bot.ts";
import type { BotChange } from "../gen/BotChange.ts";
import type { BotKey } from "../gen/BotKey.ts";
import type { BotList } from "../gen/BotList.ts";
import type { BotRemoved } from "../gen/BotRemoved.ts";
import type { CreateBot } from "../gen/CreateBot.ts";
import type { CreateCredential } from "../gen/CreateCredential.ts";
import type { CreateGrant } from "../gen/CreateGrant.ts";
import type { CreateIcon } from "../gen/CreateIcon.ts";
import type { CreateWorkspaceInvite } from "../gen/CreateWorkspaceInvite.ts";
import type { CredentialCreated } from "../gen/CredentialCreated.ts";
import type { CredentialList } from "../gen/CredentialList.ts";
import type { CustomStyles } from "../gen/CustomStyles.ts";
import type { GrantList } from "../gen/GrantList.ts";
import type { IntegrationsHealth } from "../gen/IntegrationsHealth.ts";
import type { PeoplePage } from "../gen/PeoplePage.ts";
import type { PersonChange } from "../gen/PersonChange.ts";
import type { PersonRemoved } from "../gen/PersonRemoved.ts";
import type { PersonRole } from "../gen/PersonRole.ts";
import type { SaveSlackCredentials } from "../gen/SaveSlackCredentials.ts";
import type { SlackDisconnected } from "../gen/SlackDisconnected.ts";
import type { SlackPersonal } from "../gen/SlackPersonal.ts";
import type { SlackPlan } from "../gen/SlackPlan.ts";
import type { SlackRun } from "../gen/SlackRun.ts";
import type { SlackRunChange } from "../gen/SlackRunChange.ts";
import type { SlackRunList } from "../gen/SlackRunList.ts";
import type { SlackRunPage } from "../gen/SlackRunPage.ts";
import type { SlackSetup } from "../gen/SlackSetup.ts";
import type { SlackSetupChange } from "../gen/SlackSetupChange.ts";
import type { StartPersonalSlackImport } from "../gen/StartPersonalSlackImport.ts";
import type { StartSlackDryRun } from "../gen/StartSlackDryRun.ts";
import type { StartSlackImport } from "../gen/StartSlackImport.ts";
import type { UpdateBot } from "../gen/UpdateBot.ts";
import type { UpdateWorkspace } from "../gen/UpdateWorkspace.ts";
import type { WelcomeAdmin } from "../gen/WelcomeAdmin.ts";
import type { WelcomeUpdate } from "../gen/WelcomeUpdate.ts";
import type { Workspace } from "../gen/Workspace.ts";
import type { WorkspaceIconList } from "../gen/WorkspaceIconList.ts";
import type { WorkspaceInvite } from "../gen/WorkspaceInvite.ts";
import type { WorkspaceInviteCreated } from "../gen/WorkspaceInviteCreated.ts";
import type { WorkspaceInviteList } from "../gen/WorkspaceInviteList.ts";
import { mutations, store } from "../store/store.ts";
import { brandingOf } from "../store/workspace.ts";
import { peoplePagesOver } from "./people-pages.ts";
import { runAction } from "./runtime.ts";
import { answeredForThisPage } from "./sign-in-change.ts";

/**
 * A workspace reply, with its name, logo and banner applied to the rail and sidebar at once (the
 * `workspace.updated` event that follows for every tab then changes nothing here).
 */
function branded(reply: Promise<Workspace>): Promise<Workspace> {
  return reply.then((next) => {
    mutations.setWorkspaceBranding(brandingOf(next));
    mutations.setUploadLimit(next.uploadLimitBytes);

    return next;
  });
}

export const admin = {
  workspace: (): Promise<Workspace> => branded(runAction(workspace())),

  updateWorkspace: (change: Partial<UpdateWorkspace>): Promise<Workspace> =>
    branded(
      runAction(
        updateWorkspace({
          afkPolicy: null,
          name: null,
          restrictRoomCreationToAdministrators: null,
          uploadLimitBytes: null,
          stageAudienceBudget: null,
          description: null,
          vanitySlug: null,
          ...change,
        }),
      ),
    ),

  setLogo: (signedId: string): Promise<Workspace> => branded(runAction(updateLogo(signedId))),

  removeLogo: (): Promise<Workspace> => branded(runAction(removeLogo())),

  setBanner: (signedId: string): Promise<Workspace> => branded(runAction(updateBanner(signedId))),

  removeBanner: (): Promise<Workspace> => branded(runAction(removeBanner())),

  resetJoinCode: (): Promise<Workspace> => runAction(resetJoinCode()),

  /** The welcome card and server guide as configured, and the rooms the editor offers. */
  welcome: (): Promise<WelcomeAdmin> => runAction(adminWelcome()),

  /**
   * Replaces the sections `update` names (the welcome's `message` and `rooms`, or the `guide`),
   * leaving the rest as stored; `Validation` names the refused field. Every client hears
   * `workspace.updated` and reads its copy again.
   */
  saveWelcome: (update: WelcomeUpdate): Promise<WelcomeAdmin> => runAction(updateWelcome(update)),

  invites: (): Promise<WorkspaceInviteList> => runAction(invites()),

  createInvite: (body: CreateWorkspaceInvite): Promise<WorkspaceInviteCreated> =>
    runAction(createInvite(body)),

  revokeInvite: (inviteId: number): Promise<WorkspaceInvite> => runAction(revokeInvite(inviteId)),

  people: (page: string | null = null): Promise<PeoplePage> => runAction(people(page)),

  setRole: (userId: number, role: PersonRole): Promise<PersonChange> =>
    runAction(updatePerson(userId, role)),

  removePerson: (userId: number): Promise<PersonRemoved> => runAction(removePerson(userId)),

  resetTwoFactor: (userId: number): Promise<PersonChange> => runAction(resetTwoFactor(userId)),

  /** Lets them link Google sign-in by email (`true`), or unlinks it (`false`). */
  setGoogleLink: (userId: number, allow: boolean): Promise<PersonChange> =>
    runAction(setGoogleLink(userId, allow)),

  customStyles: (): Promise<CustomStyles> => runAction(customStyles()),

  updateCustomStyles: (css: string | null): Promise<CustomStyles> =>
    runAction(updateCustomStyles(css)).then((styles) => {
      mutations.setWorkspaceStyles(styles.css);

      return styles;
    }),

  icons: (): Promise<WorkspaceIconList> => runAction(icons()),

  createIcon: (body: CreateIcon): Promise<WorkspaceIconList> => runAction(createIcon(body)),

  destroyIcon: (iconId: number): Promise<WorkspaceIconList> => runAction(destroyIcon(iconId)),

  auditLog: (filters: AuditLogFilters, page: string | null = null): Promise<AuditLogPage> =>
    runAction(auditLog(filters, page)),

  integrationsHealth: (): Promise<IntegrationsHealth> => runAction(integrationsHealth()),
};

/** The chat bot pages: plain promises over the S7 bot endpoints, failing as `admin`'s do. */
export const bots = {
  list: (): Promise<BotList> => runAction(botList()),

  create: (body: CreateBot): Promise<BotKey> => runAction(createBot(body)),

  get: (botId: number): Promise<Bot> => runAction(bot(botId)),

  update: (botId: number, change: Partial<UpdateBot>): Promise<BotChange> =>
    runAction(
      updateBot(botId, {
        name: null,
        iconName: null,
        webhookUrl: null,
        avatar: null,
        agent: null,
        ...change,
      }),
    ),

  remove: (botId: number): Promise<BotRemoved> => runAction(removeBot(botId)),

  suspend: (botId: number): Promise<BotChange> => runAction(suspendBot(botId)),

  resetKey: (botId: number): Promise<BotKey> => runAction(resetBotKey(botId)),

  resetSigningSecret: (botId: number): Promise<BotChange> => runAction(resetSigningSecret(botId)),

  connectGithub: (botId: number, accessToken: string): Promise<BotChange> =>
    runAction(connectGithub(botId, accessToken)),

  disconnectGithub: (botId: number): Promise<BotChange> => runAction(disconnectGithub(botId)),

  credentials: (botId: number): Promise<CredentialList> => runAction(credentials(botId)),

  issueCredential: (botId: number, body: CreateCredential): Promise<CredentialCreated> =>
    runAction(issueCredential(botId, body)),

  revokeCredential: (botId: number, credentialId: number): Promise<CredentialList> =>
    runAction(revokeCredential(botId, credentialId)),

  grants: (botId: number): Promise<GrantList> => runAction(grants(botId)),

  grant: (botId: number, body: CreateGrant): Promise<GrantList> =>
    runAction(createGrant(botId, body)),

  revokeGrant: (botId: number, grantId: number): Promise<GrantList> =>
    runAction(revokeGrant(botId, grantId)),
};

/**
 * The Slack importer: plain promises over the S7 Slack endpoints, failing as `admin`'s do. `admin`
 * picks the administrator's workspace pages or the person's own, where both exist.
 */
export const slack = {
  setup: (): Promise<SlackSetup> => runAction(slackSetup()),

  saveCredentials: (body: SaveSlackCredentials): Promise<SlackSetupChange> =>
    runAction(saveSlackCredentials(body)),

  removeCredentials: (): Promise<SlackSetupChange> => runAction(removeSlackCredentials()),

  runs: (): Promise<SlackRunList> => runAction(slackRuns()),

  dryRun: (body: StartSlackDryRun): Promise<SlackRunChange> => runAction(startSlackDryRun(body)),

  runPage: (runId: number, page: number | null = null): Promise<SlackRunPage> =>
    runAction(slackRunPage(runId, page)),

  status: (admin: boolean, runId: number): Promise<SlackRun> =>
    runAction(slackRunStatus(admin, runId)),

  plan: (runId: number): Promise<SlackPlan> => runAction(slackPlan(runId)),

  startImport: (runId: number, body: StartSlackImport): Promise<SlackRunChange> =>
    runAction(startSlackImport(runId, body)),

  catchUp: (runId: number): Promise<SlackRunChange> => runAction(startSlackCatchUp(runId)),

  cancel: (admin: boolean, runId: number): Promise<SlackRunChange> =>
    runAction(cancelSlackRun(admin, runId)),

  undo: (admin: boolean, runId: number): Promise<SlackRunChange> =>
    runAction(undoSlackRun(admin, runId)),

  personal: (): Promise<SlackPersonal> => runAction(personalSlack()),

  startPersonal: (body: StartPersonalSlackImport): Promise<SlackRunChange> =>
    runAction(startPersonalSlack(body)),

  disconnect: (): Promise<SlackDisconnected> => runAction(disconnectSlack()),
};

/** The people pages over the S7 people endpoints; see `peoplePagesOver`. */
export const peoplePages = peoplePagesOver({
  directory: () => runAction(peopleDirectory()),
  profile: (userId) => runAction(personProfile(userId)),
  setBanned: (userId, banned) => runAction(setBanned(userId, banned)),
  setDndAllowance: (userId, allowed) => runAction(setDndAllowance(userId, allowed)),
});

/**
 * Every `/blocks` reply and ignore write reply passes here: one the server answered for someone
 * else (another sign-in) stops the page for its reload and never settles, so nothing of theirs
 * reaches the store or the screen meanwhile.
 */
function forThisPage<Reply extends { readonly viewerId: number }>(reply: Reply): Promise<Reply> {
  return answeredForThisPage(reply.viewerId)
    ? Promise.resolve(reply)
    : new Promise<Reply>(() => undefined);
}

/**
 * Ignoring someone. Each reply lands in the store first, so every timeline collapses (or opens)
 * their messages at once; the sync echo of the same change is then dropped by its revision, and
 * refreshes the sidebar's counts.
 */
export const ignores = {
  /** The people you ignore, oldest first; the snapshot also brings the store up to date. */
  list: (): Promise<BlockList> => {
    const coveredAt = store.getState().ignores.invalidatedAt;

    return runAction(blocks())
      .then(forThisPage)
      .then((snapshot) => {
        mutations.mergeUsers(snapshot.people.map((person) => person.user));
        mutations.setBlocks(snapshot, coveredAt);

        return snapshot;
      });
  },

  /** Ignores them (`true`) or stops (`false`). */
  set: (userId: number, ignored: boolean): Promise<BlockState> =>
    runAction(setIgnored(userId, ignored))
      .then(forThisPage)
      .then((change) => {
        mutations.applyBlockState(change);

        return change;
      }),
};
