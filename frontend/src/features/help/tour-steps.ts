/**
 * The product tour's five steps, from classic's tour controller (sidebar, composer, huddles, the
 * quick switcher, the shortcuts sheet), with the same titles and copy except where classic named
 * its own UI: the shortcuts sheet is ⌘/ here, not ?, and the help items live in your account
 * menu. Each step lists where its anchor may be, most specific first; the first one on screen
 * wins, and a step with none on screen (a phone's conversation list has no composer, a room on a
 * phone has no sidebar) shows the same copy as a sheet instead.
 */
export interface TourStep {
  readonly id: "sidebar" | "composer" | "huddles" | "switcher" | "shortcuts";
  readonly title: string;
  readonly body: string;
  readonly anchors: readonly string[];
}

export const TOUR_STEPS: readonly TourStep[] = [
  {
    id: "sidebar",
    title: "Your rooms live here",
    body: "The sidebar lists every room you belong to, with unread badges. Star the ones you live in to pin them near the top.",
    anchors: [".sidebar"],
  },
  {
    id: "composer",
    title: "Write below the messages",
    body: "The composer posts to the open room. Type @ to mention someone, and use the toolbar for files, emoji, and Drive attachments.",
    anchors: [".composer"],
  },
  {
    id: "huddles",
    title: "Talk it out in a huddle",
    body: "Voice and video huddles start right from the room header, with screen sharing when you need it. The same button joins when a call is live.",
    anchors: [".room-header .huddle-launcher"],
  },
  {
    id: "switcher",
    title: "Jump anywhere with Ctrl+K",
    body: "Press Ctrl+K (or Cmd+K on a Mac) to open the room switcher and hop between conversations without touching the mouse.",
    anchors: [".sidebar-jump"],
  },
  {
    id: "shortcuts",
    title: "Shortcuts live under Ctrl+/",
    body: "Press Ctrl+/ (or Cmd+/ on a Mac) to see every keyboard shortcut. The Help items in your account menu hold them too, and restart this tour whenever you like.",
    anchors: [".sidebar-you-button", ".rail-you .rail-button"],
  },
];
