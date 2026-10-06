import { Controller } from "@hotwired/stimulus"
import MessageFormatter, { ThreadStyle } from "models/message_formatter"

// Formats messages on pages without the room shell (the standalone thread
// and message pages): own/mentioned/threaded classes, day separators, and
// code highlighting. Unlike the messages and search-results controllers it
// owns no scrolling, pagination, or streams.
export default class extends Controller {
  static targets = [ "message" ]
  static classes = [ "firstOfDay", "formatted", "me", "mentioned", "threaded" ]

  #formatter

  initialize() {
    this.#formatter = new MessageFormatter(Current.user.id, {
      firstOfDay: this.firstOfDayClass,
      formatted: this.formattedClass,
      me: this.meClass,
      mentioned: this.mentionedClass,
      threaded: this.threadedClass,
    })
  }

  messageTargetConnected(target) {
    this.#formatter.format(target, ThreadStyle.thread)
    const body = target.querySelector("[data-reply-target='body']")
    if (body) this.#formatter.formatBody(body)
  }
}
