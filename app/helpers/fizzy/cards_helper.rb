module Fizzy::CardsHelper
  # Cards referenced by a message, sorted by account and number. The
  # container renders only lazy per-viewer frames (no card content), so
  # it stays fragment-cacheable across viewers; each frame loads through
  # the card endpoint with the viewer's own token.
  def fizzy_cards_for(message)
    # Sorting in Ruby rather than with an `order` scope, because applying a
    # scope to an association builds a fresh relation and so ignores the rows
    # `with_rendering_details` already preloaded — one extra query per message
    # rendered. (Same reason `ordered_boosts` exists.)
    message.fizzy_cards.sort_by { |card| [ card.account_id, card.number ] }
  end

  # Frame ids for the per-viewer card frames. The cards container renders
  # the empty frame with this id; the card endpoint recomputes the same
  # id from its message param so the loaded frame replaces the placeholder.
  def fizzy_card_frame_id(card, message_id:)
    dom_id(card, "card_for_message_#{message_id}")
  end

  # Status pill for a cached card payload: Closed, Postponed, the column
  # name, or Maybe? for untriaged cards.
  def fizzy_card_status_label(payload)
    payload = payload.presence || {}
    if payload["closed"]
      "Closed"
    elsif payload["postponed"]
      "Postponed"
    else
      payload.dig("column", "name").presence || "Maybe?"
    end
  end

  def fizzy_card_status_kind(payload)
    payload = payload.presence || {}
    if payload["closed"]
      "closed"
    elsif payload["postponed"]
      "postponed"
    elsif payload.dig("column", "name").present?
      "column"
    else
      "triage"
    end
  end

  # "2/3 steps" for cards with steps, nil otherwise.
  def fizzy_card_steps_label(payload)
    steps = Array(payload&.dig("steps"))
    return if steps.empty?

    done = steps.count { |step| step["completed"] }
    "#{done}/#{steps.size} steps"
  end

  def fizzy_card_last_active_at(payload)
    Time.zone.parse(payload["last_active_at"].to_s)
  rescue ArgumentError, TypeError
    nil
  end
end
