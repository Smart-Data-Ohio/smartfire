import BaseAutocompleteHandler from "lib/autocomplete/base_autocomplete_handler"
import { escapeHTML } from "helpers/string_helpers"

// Slash-command picker: active only when the word being typed starts at
// the very beginning of the composer ("/" opens it, matching Discord and
// Slack). Committing "/poll" inserts the command plus a trailing space
// so the author can keep typing arguments.
export default class MarkdownSlashCommandsAutocompleteHandler extends BaseAutocompleteHandler {
  get pattern() {
    return /^\/(.*?)$/
  }

  // The picker follows the first word while the caret stays on the first
  // line: it opens on "/", filters as the word grows, and closes once an
  // argument starts (matchQueryAndTerminatorForWord rejects words with a
  // space, so the context update deactivates it) or on a later line.
  // Later words starting with "/" never open it.
  shouldAutocompleteWithContentAndPosition(content, position) {
    const before = content.slice(0, position)
    if (before.includes("\n")) return false
    return before.split(/\s/)[0]?.startsWith("/") === true
  }

  matchQueryAndTerminatorForWord(word) {
    // A space ends the command word: once "/command " has arguments the
    // picker has nothing to complete, so the context goes inactive and
    // the update deactivates the picker instead of re-querying for the
    // whole line. (Space stays a non-boundary character below so typing
    // it never commits the top match; this only ends the match.)
    if (/\s/.test(word)) return undefined
    return super.matchQueryAndTerminatorForWord(word)
  }

  // Once "/command " is chosen — arguments started or just a trailing
  // space — Enter submits the form instead of committing the suggestion.
  // The picker update is debounced, so without this a stale active state
  // would swallow the submit while the deactivating update is pending.
  shouldSubmitOnReturnKey() {
    return /^\s*\/[^\s]+\s/.test(this.element.value)
  }

  // Retyping "/" (the "//" escape, or a fresh "/" after clearing) must
  // never cancel the picker or swallow the keystroke: the context
  // update hides results naturally when the query matches nothing.
  shouldCancelOnKey(_value) {
    return false
  }

  characterMatchesWordBoundary(character) {
    // Space starts the command's arguments; it never commits the top
    // match the way it finishes a mention or an emoji shortcode.
    return character !== " " && /[\s\uFFFC]/.test(character)
  }

  insertAutocompletable(autocompletable, range, terminator) {
    if (!autocompletable?.value) return

    const replacement = `/${autocompletable.value} `
    this.element.setRangeText(replacement, range[0], range[1], "end")
    this.element.dispatchEvent(new Event("input", { bubbles: true }))
  }

  fetchResultsForQuery(query, callback) {
    this.loadAutocompletables(query, () => {
      const autocompletables = this.autocompletablesMatchingQuery(query)
      callback(this.#renderSuggestions(autocompletables))
    })
  }

  didShowResults(selectElement) {
    selectElement.classList.add("markdown-autocomplete")
  }

  #renderSuggestions(autocompletables) {
    return autocompletables.map(command => {
      const name = escapeHTML(`/${command.name}`)
      const description = escapeHTML(command.description || "")
      const hint = command.arg_hint ? ` <span class="slash-command__hint">${escapeHTML(command.arg_hint)}</span>` : ""
      const agent = command.agent ? ` <small>by ${escapeHTML(command.agent)}</small>` : ""

      return `
        <suggestion-option class="autocomplete__item flex align-center gap unpad" role="option" value="${escapeHTML(command.value)}">
          <button type="button" class="autocomplete__btn btn btn--borderless btn--transparent min-width flex-item-grow justify-start">
            <span class="autocompletable__name"><strong>${name}</strong>${hint}</span>
            <small>${description}${agent}</small>
          </button>
        </suggestion-option>
      `
    }).join("")
  }
}
