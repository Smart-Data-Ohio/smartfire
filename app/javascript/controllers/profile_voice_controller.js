import { Controller } from "@hotwired/stimulus"

// The huddle panel is turbo-permanent, so a saved Calls setting would sit
// stale on it until the next full page load. After a successful save, copy
// the saved values onto the panel: the next join — and the shortcuts, which
// read the live values — pick them up immediately, with no reload.
export default class extends Controller {
  static targets = [ "voiceMode", "pushToTalkKey" ]

  sync(event) {
    if (event?.detail && event.detail.success === false) return
    if (event?.target instanceof HTMLFormElement && !event.target.contains(this.element)) return

    const panel = document.getElementById("channel-huddle")
    if (!panel) return

    if (this.hasVoiceModeTarget) panel.dataset.huddleVoiceModeValue = this.voiceModeTarget.value

    // The server strips the key and falls back to the backtick, so an
    // emptied field syncs the backtick instead of an empty string.
    if (this.hasPushToTalkKeyTarget) {
      panel.dataset.huddlePushToTalkKeyValue = this.pushToTalkKeyTarget.value.trim() || "`"
    }
  }
}
