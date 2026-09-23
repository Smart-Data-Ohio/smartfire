import SuggestionResultsController from "lib/autocomplete/suggestion_results_controller"
import SuggestionContext from "lib/autocomplete/suggestion_context"

export default class SuggestionController {
  #active     = false
  #canceled   = false
  #committing = false
  #context
  #pendingUpdate = null
  #resultsController

  constructor(delegate) {
    this.delegate = delegate
    this.commitSuggestion = this.commitSuggestion.bind(this)
    this.characterMatchesWordBoundary = this.characterMatchesWordBoundary.bind(this)
    this.matchQueryAndTerminatorForWord = this.matchQueryAndTerminatorForWord.bind(this)
    this.didPressKey = this.didPressKey.bind(this)
    this.didResizeWindow = this.didResizeWindow.bind(this)
    this.didScrollWindow = this.didScrollWindow.bind(this)
    this.#installKeyboardListener()
    this.#installResizeListeners()
  }

  updateWithContentAndPosition(content, position) {
    // A fast typist reaches the next query while the click commit's flash
    // animation still runs. Dropping that update strands the input with no
    // suggestions and nothing left to trigger them, so remember the latest
    // one and replay it once the commit finishes deactivating.
    if (this.#committing) {
      this.#pendingUpdate = [ content, position ]
      return
    }
    const previousContext = this.#context
    this.#context = new SuggestionContext(this, content, position)

    if (!this.#context.isEqualTo(previousContext)) {
      if (this.#context.isTerminated()) {
        return this.commitSuggestion()
      } else if (this.#context.isActive()) {
        return this.#activateSuggestion()
      } else {
        return this.#deactivateSuggestion()
      }
    }
  }

  hideResults() {
    return this.#resultsController.hide()
  }

  destroy() {
    this.#pendingUpdate = null
    this.#uninstallResultsController()
    this.#uninstallResizeListeners()
    this.#uninstallKeyboardListener()
    this.#syncComboboxState()
  }

  commitSuggestion({withTerminator} = {}) {
    if (this.#committing || this.#canceled || !this.#active) { return false }

    const values = this.selectedValues
    if (values.length == 0) { return false }

    const range = [this.#context.startPosition, this.#context.endPosition]
    const terminator = (withTerminator != null ? withTerminator : this.#context.terminator) || " "

    this.#committing = true
    this.#resultsController.flashSelection(() => {
      this.#committing = false
      this.#didCommitValuesAtRangeWithTerminator(values, range, terminator)
      // With a query already waiting, skip the hide animation: deactivating
      // synchronously lets the replay install a fresh results list instead
      // of fetching into one the animation is about to destroy.
      if (this.#pendingUpdate) this.#deactivateSuggestion()
      else this.#deactivateSuggestionWithAnimation()
      this.#replayPendingUpdate()
    })

    if (values.length > 1) {
      this.#willCommitValuesAtRangeWithTerminator(values, range, terminator, { editor: this.delegate.editor })
    } else {
      this.delegate.willCommitValueAtRangeWithTerminator?.(values[0], range, terminator)
    }
    return true
  }

  get selectedValues() {
    const value = this.#resultsController.getSelectedValue()
    if (!value) { return [] }

    const valueOnlyHasCommaSeparatedNumbers = /^\d+(,\d+)*$/.test(value)
    return valueOnlyHasCommaSeparatedNumbers ? value.split(",") : [value]
  }

  isActive() {
    return this.#active
  }

  isCanceled() {
    return this.#canceled
  }

  #activateSuggestion() {
    if (!this.#canceled) {
      this.#active = true
      this.#installResultsController()
      return this.#updateResults(() => {
        if (this.#resultsController.hasResults()) {
          return this.#displayResults()
        } else {
          return this.hideResults()
        }
      })
    }
  }

  #deactivateSuggestionWithAnimation() {
    this.#hideResultsWithAnimation(() => {
      this.#deactivateSuggestion()
    })
  }

  #deactivateSuggestion() {
    this.#uninstallResultsController()
    this.#active = false
    this.#canceled = false
    this.#syncComboboxState()
  }

  #replayPendingUpdate() {
    const pending = this.#pendingUpdate
    this.#pendingUpdate = null
    if (pending) this.updateWithContentAndPosition(...pending)
  }

  #cancelSuggestion() {
    if (this.#active) {
      this.#deactivateSuggestion()
      this.#canceled = true
    }
  }

  #resumeSuggestion() {
    if (this.#canceled) {
      this.#canceled = false
      return this.#activateSuggestion()
    }
  }

  #willCommitValuesAtRangeWithTerminator(values, range, terminator, { editor }) {
    editor?.setSelectedRange(range) && editor?.deleteInDirection("forward") // Delete user autocomplete input

    values.forEach((value) => {
      this.delegate.willCommitValueAtRangeWithTerminator?.(value, null, terminator)
      range = this.#advanceRangeForNextValue(range)
    })
  }

  #didCommitValuesAtRangeWithTerminator(values, range, terminator) {
    values.forEach((value) => {
      this.delegate.didCommitValueAtRangeWithTerminator?.(value, range, terminator)
      range = this.#advanceRangeForNextValue(range)
    })
  }

  #advanceRangeForNextValue(range) {
    const startPosition = range[1] + 1
    return Array(startPosition, startPosition + 1)
  }

  #displayResults() {
    if (this.#active) {
      const offsets = this.delegate.getOffsetsAtPosition(this.#context.startPosition)
      const placement = this.delegate.getResultsPlacement?.()
      return this.#resultsController.displayAtOffsets(offsets, {placement})
    }
  }

  #updateResults(callback) {
    const query = this.#context?.query
    return this.delegate.fetchResultsForQuery(query, results => {
      if ((this.#resultsController != null) && (query === this.#context?.query)) {
        this.#resultsController.updateResults(results)
        this.#syncComboboxState()
        return callback?.()
      }
    })
  }

  #hideResultsWithAnimation(callback) {
    return this.#resultsController?.hideWithAnimation(callback)
  }

  // Suggestion context delegate

  characterMatchesWordBoundary(character) {
    if (this.delegate.characterMatchesWordBoundary != null) {
      return this.delegate.characterMatchesWordBoundary(character)
    } else {
      return /[\s\uFFFC]/.test(character)
    }
  }

  matchQueryAndTerminatorForWord(word) {
    return this.delegate.matchQueryAndTerminatorForWord(word)
  }

  // Results controller delegate

  didClickOption(option) {
    return setTimeout(this.commitSuggestion, 100)
  }

  didShowResults(element) {
    this.hidden = false
    this.#syncComboboxState()
    return this.delegate.didShowResults?.(element)
  }

  didHideResults(element) {
    this.hidden = true
    this.#syncComboboxState()
    return this.delegate.didHideResults?.(element)
  }

  // Keyboard events

  didPressKey(event) {
    if (this.#committing || event.isComposing || event.keyCode === 229) { return }

    let result
    switch (event.keyCode) {
      case 9:
        result = this.#didPressTabKey()
        break
      case 10: case 13:
        result = this.#didPressReturnKey()
        break
      case 27:
        result = this.#didPressEscapeKey()
        break
      case 32:
        result = this.#didPressSpaceKey()
        break
      case 38:
        result = this.#didPressUpKey()
        break
      case 40:
        result = this.#didPressDownKey()
        break
      default:
        result = this.#didPressKeyWithValue(event.key)
    }

    if (result === false) {
      event.preventDefault()
      return event.stopPropagation()
    }
  }

  #didPressTabKey() {
    if (this.#active) {
      if (this.hidden) {
        this.#displayResults()
        return false
      } else if (!this.#committing) {
        if (this.commitSuggestion()) {
          return false
        }
      }
    } else if (this.#canceled) {
      this.#resumeSuggestion()
      return false
    }
  }

  #didPressReturnKey() {
    if (this.#active) {
      // A delegate may yield Enter to the form: the slash picker does
      // once "/command " is chosen, so a stale active state never
      // swallows the submit while the debounced update is still pending.
      if (this.delegate.shouldSubmitOnReturnKey?.() === true) {
        this.#deactivateSuggestion()
        return
      }
      this.commitSuggestion()
      return false
    }
  }

  #didPressEscapeKey() {
    if (this.#active) {
      this.#cancelSuggestion()
      return false
    }
  }

  #didPressSpaceKey() {
    if (this.#active && this.#spaceMatchesWordBoundary()) {
      if (this.commitSuggestion()) {
        return false
      }
    }
  }

  #didPressUpKey() {
    if (this.#active) {
      this.#resultsController.selectUp()
      this.#syncComboboxState()
      return false
    }
  }

  #didPressDownKey() {
    if (this.#active) {
      this.#resultsController.selectDown()
      this.#syncComboboxState()
      return false
    }
  }

  #didPressKeyWithValue(value) {
    if (this.#active && (value != null) && !this.hidden) {
      // Retyping the trigger character cancels mentions and emoji, but a
      // delegate may opt out (the slash picker: "/" starts an escape or
      // a fresh command, and swallowing it would corrupt the composer).
      if (typeof this.delegate.shouldCancelOnKey === "function" && !this.delegate.shouldCancelOnKey(value)) return
      const result = this.matchQueryAndTerminatorForWord(value)
      if (result?.query === "") {
        this.#cancelSuggestion()
        return false
      }
    }
  }

  // Scroll and resize events

  didResizeWindow() {
    if (this.#active) {
      return this.hideResults()
    }
  }

  didScrollWindow(event) {
    if (this.#active && (event.target === document)) {
      return this.hideResults()
    }
  }

  // Private

  #installKeyboardListener() {
    window.addEventListener("keydown", this.didPressKey, true)
  }

  #uninstallKeyboardListener() {
    window.removeEventListener("keydown", this.didPressKey, true)
  }

  #installResizeListeners() {
    window.addEventListener("resize", this.didResizeWindow, true)
    window.addEventListener("scroll", this.didScrollWindow, true)
  }

  #uninstallResizeListeners() {
    window.removeEventListener("resize", this.didResizeWindow, true)
    window.removeEventListener("scroll", this.didScrollWindow, true)
  }

  #installResultsController() {
    if (!this.#resultsController) {
      this.#resultsController = new SuggestionResultsController({ id: this.delegate.getSuggestionsIdentifier() })
    }
    this.#resultsController.delegate = this
  }

  #uninstallResultsController() {
    this.#resultsController?.destroy()
    this.#resultsController = null
  }

  #spaceMatchesWordBoundary() {
    return this.characterMatchesWordBoundary(" ")
  }

  // Keeps the input's combobox semantics in step with the listbox: expanded
  // while visible results exist, tracking the selected option otherwise.
  #syncComboboxState() {
    const element = this.delegate?.element
    if (!element || typeof element.setAttribute !== "function") return

    const resultsController = this.#resultsController
    const expanded = this.#active && !!resultsController?.visible && resultsController.hasResults()

    element.setAttribute("aria-expanded", String(expanded))

    if (expanded) {
      element.setAttribute("aria-controls", resultsController.selectElement.id)
      const selected = resultsController.selectElement.selectedOption
      if (selected?.id) {
        element.setAttribute("aria-activedescendant", selected.id)
      } else {
        element.removeAttribute("aria-activedescendant")
      }
    } else {
      element.removeAttribute("aria-controls")
      element.removeAttribute("aria-activedescendant")
    }
  }
}
