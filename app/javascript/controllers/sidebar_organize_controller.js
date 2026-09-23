import { Controller } from "@hotwired/stimulus"

// Drag-and-drop sidebar organizing: reorder favourites within their
// section, and drag channels into categories (or back to Channels to
// unassign). Favourites stay out of categories — unfavourite first —
// and every drop has a keyboard equivalent in the room menu.
export default class extends Controller {
  #dragRow
  #observer

  connect() {
    this.onDragStart = this.#onDragStart.bind(this)
    this.onDragOver = this.#onDragOver.bind(this)
    this.onDragLeave = this.#onDragLeave.bind(this)
    this.onDrop = this.#onDrop.bind(this)
    this.onDragEnd = this.#onDragEnd.bind(this)

    this.#markDraggableRows()
    // Rows streamed in later (new rooms, visibility changes) join in.
    this.#observer = new MutationObserver(() => this.#markDraggableRows())
    this.#observer.observe(this.element, { childList: true, subtree: true })

    this.element.addEventListener("dragstart", this.onDragStart)
    this.element.addEventListener("dragover", this.onDragOver)
    this.element.addEventListener("dragleave", this.onDragLeave)
    this.element.addEventListener("drop", this.onDrop)
    this.element.addEventListener("dragend", this.onDragEnd)
  }

  disconnect() {
    this.#observer?.disconnect()
    this.#observer = null
    this.element.removeEventListener("dragstart", this.onDragStart)
    this.element.removeEventListener("dragover", this.onDragOver)
    this.element.removeEventListener("dragleave", this.onDragLeave)
    this.element.removeEventListener("drop", this.onDrop)
    this.element.removeEventListener("dragend", this.onDragEnd)
  }

  #onDragStart(event) {
    const row = event.target.closest?.("a[data-room-id]")
    if (!row || !row.draggable) return
    this.#dragRow = row
    if (event.dataTransfer) {
      event.dataTransfer.setData("text/plain", row.dataset.roomId)
      event.dataTransfer.effectAllowed = "move"
    }
  }

  #onDragOver(event) {
    const target = this.#dropTargetFrom(event.target)
    if (!target || !this.#canDrop(target)) return
    event.preventDefault()
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move"
    target.classList.add("sidebar-drop-target")
  }

  #onDragLeave(event) {
    const target = this.#dropTargetFrom(event.target)
    if (target && !target.contains(event.relatedTarget)) target.classList.remove("sidebar-drop-target")
  }

  async #onDrop(event) {
    const target = this.#dropTargetFrom(event.target)
    if (!target || !this.#canDrop(target)) return
    event.preventDefault()

    const roomId = this.#dragRow.dataset.roomId
    let response

    if (target.id === "favorite_rooms") {
      response = await this.#request(`/rooms/${roomId}/favorite`, "PATCH", { position: this.#dropPosition(target, event.clientY) })
    } else {
      const categoryId = target.dataset.categoryDrop || ""
      if (!categoryId && !this.#dragRow.dataset.menuCategoryId) return
      if (categoryId && categoryId === this.#dragRow.dataset.menuCategoryId) return
      response = await this.#request(`/rooms/${roomId}/category`, "PATCH", { room_category_id: categoryId })
    }

    if (response?.ok) document.getElementById("user_sidebar")?.reload()
  }

  #onDragEnd() {
    this.#dragRow = null
    this.element.querySelectorAll(".sidebar-drop-target").forEach(target => target.classList.remove("sidebar-drop-target"))
  }

  // Internal

  #markDraggableRows() {
    this.element.querySelectorAll("#favorite_rooms a[data-room-id], #shared_rooms a[data-room-id], [data-category-drop] a[data-room-id]")
      .forEach(row => { row.draggable = true })
  }

  #dropTargetFrom(node) {
    return node.closest?.("#favorite_rooms, [data-category-drop]")
  }

  // Favourites reorder among themselves; category sections (and the
  // Channels section, which unassigns) take non-favourited channels.
  #canDrop(target) {
    if (!this.#dragRow) return false

    if (target.id === "favorite_rooms") {
      return this.#dragRow.closest("#favorite_rooms") !== null
    }
    return this.#dragRow.dataset.menuCategorizable === "true" && this.#dragRow.dataset.menuFavorited !== "true"
  }

  #dropPosition(container, clientY) {
    const rows = Array.from(container.querySelectorAll("a[data-room-id]")).filter(row => row !== this.#dragRow)

    for (let index = 0; index < rows.length; index++) {
      const rect = rows[index].getBoundingClientRect()
      if (clientY < rect.top + rect.height / 2) return index
    }
    return rows.length
  }

  async #request(url, method, body) {
    return await fetch(url, {
      method,
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        "X-CSRF-Token": document.querySelector("meta[name='csrf-token']")?.content || "",
      },
      body: JSON.stringify(body),
    }).catch(() => null)
  }
}
