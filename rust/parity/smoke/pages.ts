// Small, human-reviewed cutover inventory. No pixel comparisons or pass thresholds.
export interface SmokePage {
  id: string
  title: string
  path: string
  anonymous?: boolean
  fullPage?: boolean
  blockMedia?: boolean
  scroll?: "bottom" | string
  scrollContainerBottom?: string
  steps?: (({ click: string } | { wait_for: string }) & { viewport?: "desktop" | "phone" })[]
}

export const pages: SmokePage[] = [
  { id: "sign-in", title: "Sign-in", path: "/session/new", anonymous: true },
  { id: "room", title: "Room timeline", path: "/rooms/{{rooms.designers}}", scroll: "bottom" },
  { id: "room-attachment", title: "Room attachment and reaction", path: "/rooms/{{rooms.designers}}/@{{messages.image}}", scroll: '.message[data-message-id="{{messages.image}}"]' },
  { id: "room-reactions", title: "Room reactions", path: "/rooms/{{rooms.designers}}/@{{messages.boosted_many}}", scroll: '.message[data-message-id="{{messages.boosted_many}}"]' },
  { id: "room-thread-count", title: "Room thread reply count", path: "/rooms/{{rooms.designers}}/@{{messages.markdown}}", scroll: '.message[data-message-id="{{messages.markdown}}"] .message__thread-indicator' },
  { id: "thread", title: "Open thread", path: "/rooms/{{rooms.designers}}/threads/{{threads.launch}}" },
  {
    id: "thread-panel", title: "Open thread panel", path: "/rooms/{{rooms.designers}}",
    steps: [
      { click: '[data-action="thread-panel#toggle"]', viewport: "desktop" },
      { click: "#header-overflow-button", viewport: "phone" },
      { click: '[data-header-overflow-forward-value=".room-header__actions .room-header__action--threads"]', viewport: "phone" },
      { wait_for: '#thread-panel .thread-panel__thread-item[data-thread-id="{{threads.launch}}"]' },
      { click: '#thread-panel .thread-panel__thread-item[data-thread-id="{{threads.launch}}"]' },
      { wait_for: "#thread-panel .composer__textarea" },
    ],
  },
  { id: "dms", title: "Direct messages", path: "/rooms/{{rooms.david_and_jason}}", scroll: "bottom" },
  { id: "profile", title: "User profile", path: "/users/{{users.david}}" },
  { id: "profile-settings", title: "User profile preferences", path: "/users/me/profile" },
  { id: "profile-settings-bottom", title: "User profile preferences (bottom)", path: "/users/me/profile", scrollContainerBottom: "#main-content" },
  { id: "agents", title: "Agents directory", path: "/agents" },
  { id: "agent", title: "One agent profile", path: "/users/{{users.bender}}" },
  { id: "agent-ledger", title: "One agent event ledger", path: "/agents/{{agents.bender_agent}}/events" },
  { id: "huddle", title: "Huddle idle", path: "/rooms/{{rooms.voice}}" },
  {
    id: "huddle-prejoin", title: "Huddle pre-join (media denied)", path: "/rooms/{{rooms.voice}}", blockMedia: true,
    steps: [
      { click: 'button[data-controller~="huddle-launcher"][aria-label="Join voice"]' },
      { wait_for: '#channel-huddle[data-state="prejoin"]' },
    ],
  },
  { id: "board", title: "Board", path: "/rooms/{{rooms.board}}" },
  { id: "board-post", title: "Board work item", path: "/rooms/{{rooms.board}}/threads/{{threads.in_progress}}" },
  {
    id: "board-post-navigation", title: "Board work item navigation", path: "/rooms/{{rooms.board}}/threads/{{threads.in_progress}}",
    steps: [
      { click: '[aria-label="Open workspace navigation"]', viewport: "phone" },
      { wait_for: "body.workspace-navigation-open", viewport: "phone" },
    ],
  },
  { id: "account-settings", title: "Account settings", path: "/account/edit" },
  { id: "account-settings-bottom", title: "Account settings (bottom)", path: "/account/edit", scrollContainerBottom: "#main-content" },
  { id: "room-settings", title: "Room settings", path: "/rooms/closeds/{{rooms.designers}}/edit" },
  { id: "room-settings-bottom", title: "Room settings (bottom)", path: "/rooms/closeds/{{rooms.designers}}/edit", scrollContainerBottom: "#main-content" },
]

export function interpolate(value: string, labels: Record<string, unknown>): string {
  return value.replace(/\{\{([^}]+)\}\}/g, (_, key: string) => {
    const found = labels[key]
    if (found === undefined) throw new Error(`seed has no label ${key}`)
    return String(found)
  })
}
