import { Controller } from "@hotwired/stimulus"
import { nextEventLoopTick } from "helpers/timing_helpers"

export default class extends Controller {
  #finishLoad = () => {}

  disconnect() {
    this.#finishLoad()
    this.#finishLoad = () => {}
  }

  unpermanize() {
    delete this.element.dataset.turboPermanent
  }

  reload() {
    this.element.reload()
  }

  load({ params: { url }, detail }) {
    let finish
    let started = false
    const followLoaded = () => {
      const loaded = this.element.loaded
      const settled = () => {
        if (this.element.loaded === loaded) finish()
      }
      Promise.resolve(loaded).then(settled, settled)
    }
    const observer = new MutationObserver(() => {
      if (started) followLoaded()
    })
    const completion = new Promise(resolve => finish = value => {
      observer.disconnect()
      resolve(value)
    })
    // Native reload can abort the old promise; follow the frame's current request.
    this.#finishLoad(completion)
    this.#finishLoad = finish
    observer.observe(this.element, { attributes: true, attributeFilter: [ "src" ] })

    nextEventLoopTick().then(() => {
      if (this.#finishLoad !== finish) return
      started = true
      this.element.src = url
      followLoaded()
    }).catch(() => finish())
    detail?.completions?.push(completion)
  }
}
