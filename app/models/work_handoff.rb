# The structured context package behind one work handoff: a summary,
# links, and open questions from the sender to the receiving agent. All
# fields are capped here; rendering escapes them (plain-text views and
# JSON only, never interpreted as HTML). The ledger and webhook payloads
# read WorkHandoff#payload, snapshotted into the event metadata at
# handoff time so delivery never depends on this row surviving.
class WorkHandoff < ApplicationRecord
  SUMMARY_LIMIT = 2000
  LINKS_LIMIT = 10
  LINK_LIMIT = 500
  OPEN_QUESTIONS_LIMIT = 10
  OPEN_QUESTION_LIMIT = 500

  belongs_to :channel_thread
  belongs_to :sender, class_name: "User"
  belongs_to :receiver_agent, class_name: "Agent"

  validates :summary, presence: true, length: { maximum: SUMMARY_LIMIT }
  validate :links_within_limits
  validate :open_questions_within_limits

  before_validation :normalize_collections

  # The handoff key in work_handed_off poll and webhook payloads, plus
  # the metadata snapshot written at handoff time.
  # The receiver rule, shared by the human handoff controller and the
  # agent handoff service: an active agent member of the thread's room
  # holding post_messages (work-owner eligibility) and manage_threads
  # (needed to work the thread through the agent API), and not the
  # current owner. Returns nil when the receiver may take the thread,
  # otherwise the validation message the caller renders as 422.
  def self.receiver_error(thread, agent)
    unless agent.is_a?(Agent) && agent.active? &&
        thread.room.memberships.exists?(user_id: agent.user_id) &&
        agent.can?(:post_messages, thread.room)
      return "Receiver must be an active agent member of this room with permission to post"
    end

    unless agent.can?(:manage_threads, thread.room)
      return "Receiver must hold the manage_threads capability in this room"
    end

    if thread.work_owner_id == agent.user_id
      return "Receiver is already the owner of this work"
    end

    nil
  end

  def payload
    {
      "id" => id,
      "summary" => summary,
      "links" => links,
      "open_questions" => open_questions,
      "sender_name" => sender&.name,
      "receiver_agent_id" => receiver_agent_id
    }
  end

  def links
    Array(super)
  end

  def open_questions
    Array(super)
  end

  private
    # Accepts arrays or newline/comma-separated strings from forms; stores
    # stripped, de-duplicated string arrays.
    def normalize_collections
      self.links = normalize_list(links)
      self.open_questions = normalize_list(open_questions)
    end

    def normalize_list(value)
      Array(value).flat_map { |item| item.to_s.split(/[\r\n]+/) }
        .map { |item| item.to_s.strip }.reject(&:blank?).uniq
    end

    def links_within_limits
      if links.size > LINKS_LIMIT
        errors.add(:links, "are limited to #{LINKS_LIMIT} per handoff")
      end

      links.each do |link|
        if link.length > LINK_LIMIT
          errors.add(:links, "must be at most #{LINK_LIMIT} characters each")
          break
        elsif !link.match?(/\Ahttps?:\/\//i)
          errors.add(:links, "must be http(s) URLs")
          break
        end
      end
    end

    def open_questions_within_limits
      if open_questions.size > OPEN_QUESTIONS_LIMIT
        errors.add(:open_questions, "are limited to #{OPEN_QUESTIONS_LIMIT} per handoff")
      end

      open_questions.each do |question|
        if question.length > OPEN_QUESTION_LIMIT
          errors.add(:open_questions, "must be at most #{OPEN_QUESTION_LIMIT} characters each")
          break
        end
      end
    end
end
