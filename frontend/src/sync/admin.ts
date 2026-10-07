/**
 * What the admin screens call: plain promises over the S7 admin endpoints. Failures reject with an
 * `ActionError`: `SudoRequired` when the classic page would ask for the password first,
 * `Validation` with the classic page's message (and `fields`) when a change is refused.
 */
import {
  auditLog,
  createIcon,
  customStyles,
  destroyIcon,
  icons,
  integrationsHealth,
  people,
  removeLogo,
  removePerson,
  resetJoinCode,
  resetTwoFactor,
  setGoogleLink,
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
import type { AuditLogFilters } from "../gen/AuditLogFilters.ts";
import type { AuditLogPage } from "../gen/AuditLogPage.ts";
import type { Bot } from "../gen/Bot.ts";
import type { BotChange } from "../gen/BotChange.ts";
import type { BotKey } from "../gen/BotKey.ts";
import type { BotList } from "../gen/BotList.ts";
import type { BotRemoved } from "../gen/BotRemoved.ts";
import type { CreateBot } from "../gen/CreateBot.ts";
import type { CreateCredential } from "../gen/CreateCredential.ts";
import type { CreateGrant } from "../gen/CreateGrant.ts";
import type { CreateIcon } from "../gen/CreateIcon.ts";
import type { CredentialCreated } from "../gen/CredentialCreated.ts";
import type { CredentialList } from "../gen/CredentialList.ts";
import type { CustomStyles } from "../gen/CustomStyles.ts";
import type { GrantList } from "../gen/GrantList.ts";
import type { IntegrationsHealth } from "../gen/IntegrationsHealth.ts";
import type { PeoplePage } from "../gen/PeoplePage.ts";
import type { PersonChange } from "../gen/PersonChange.ts";
import type { PersonRemoved } from "../gen/PersonRemoved.ts";
import type { PersonRole } from "../gen/PersonRole.ts";
import type { UpdateBot } from "../gen/UpdateBot.ts";
import type { UpdateWorkspace } from "../gen/UpdateWorkspace.ts";
import type { Workspace } from "../gen/Workspace.ts";
import type { WorkspaceIconList } from "../gen/WorkspaceIconList.ts";
import { runAction } from "./runtime.ts";

export const admin = {
  workspace: (): Promise<Workspace> => runAction(workspace()),

  updateWorkspace: (change: Partial<UpdateWorkspace>): Promise<Workspace> =>
    runAction(
      updateWorkspace({ name: null, restrictRoomCreationToAdministrators: null, ...change }),
    ),

  setLogo: (signedId: string): Promise<Workspace> => runAction(updateLogo(signedId)),

  removeLogo: (): Promise<Workspace> => runAction(removeLogo()),

  resetJoinCode: (): Promise<Workspace> => runAction(resetJoinCode()),

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
    runAction(updateCustomStyles(css)),

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
