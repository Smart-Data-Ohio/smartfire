module Fizzy
  # One agent-requested Fizzy write action: create, comment, move, close,
  # or reopen a card. Validates the agent's input, builds the approval's
  # action/summary/payload, and performs the Fizzy call through the same
  # Client methods the member controllers use. Never touches Fizzy until
  # #perform, which only runs after a human approves.
  class AgentCardAction
    include ActiveModel::Model

    KINDS = %w[ create comment move close reopen ].freeze
    SUMMARY_EXCERPT_CHARS = 120

    # The approval payload column holds 4 KB, so bodies stay well under it;
    # the approval validation remains the backstop.
    MAX_TITLE_CHARS = 500
    MAX_BODY_CHARS = 3500

    ACCOUNT_ID_FORMAT = /\A[A-Za-z0-9_-]+\z/

    attr_accessor :account_id, :kind, :board_id, :number, :column_id, :title, :description, :body

    validates :kind, inclusion: { in: KINDS, message: "must be one of: create, comment, move, close, reopen" }
    validates :account_id, presence: true, format: { with: ACCOUNT_ID_FORMAT, message: "is invalid" }
    validates :board_id, format: { with: ACCOUNT_ID_FORMAT, message: "is invalid" }, allow_blank: true
    validates :column_id, format: { with: ACCOUNT_ID_FORMAT, message: "is invalid" }, allow_blank: true
    validate :kind_requirements
    validate :text_length_within_payload

    def self.from_payload(payload:)
      payload = payload.is_a?(Hash) ? payload : {}
      new(
        account_id: payload["account_id"],
        kind: payload["kind"],
        board_id: payload["board_id"],
        number: payload["number"],
        column_id: payload["column_id"],
        title: payload["title"],
        description: payload["description"],
        body: payload["body"]
      )
    end

    def normalized_number
      number.to_s.match?(/\A\d+\z/) ? number.to_i : nil
    end

    def normalized_title
      title.to_s.strip.presence
    end

    def normalized_description
      description.to_s.strip.presence
    end

    def normalized_body
      body.to_s.strip.presence
    end

    def normalized_board_id
      board_id.to_s.strip.presence
    end

    def normalized_column_id
      column_id.to_s.strip.presence
    end

    def action_name
      "fizzy.#{kind}"
    end

    def summary
      case kind.to_s
      when "create"
        "Create Fizzy card in board #{normalized_board_id} (account #{account_id}): #{normalized_title.to_s[0, SUMMARY_EXCERPT_CHARS]}".truncate(500)
      when "comment"
        "Comment on Fizzy card ##{normalized_number} (account #{account_id}): #{normalized_body.to_s[0, SUMMARY_EXCERPT_CHARS]}"
      when "move"
        "Move Fizzy card ##{normalized_number} (account #{account_id}) to column #{normalized_column_id}"
      when "close"
        "Close Fizzy card ##{normalized_number} (account #{account_id})"
      when "reopen"
        "Reopen Fizzy card ##{normalized_number} (account #{account_id})"
      end
    end

    def payload_hash
      {
        "account_id" => account_id.to_s,
        "kind" => kind.to_s,
        "board_id" => (normalized_board_id if kind.to_s == "create"),
        "number" => (normalized_number unless kind.to_s == "create"),
        "column_id" => (normalized_column_id if kind.to_s == "move"),
        "title" => (normalized_title if kind.to_s == "create"),
        "description" => (normalized_description if kind.to_s == "create"),
        "body" => (normalized_body if kind.to_s == "comment")
      }
    end

    def payload_json
      payload_hash.to_json
    end

    # Performs the Fizzy call with the agent owner's linked token. Returns
    # the parsed response for create and comment (callers read url from
    # it), true for move, close, and reopen. Raises the same Client
    # errors the member controllers rescue.
    def perform(client)
      case kind.to_s
      when "create"
        client.create_card(account_id, normalized_board_id, title: normalized_title, description: normalized_description)
      when "comment"
        client.create_comment(account_id, normalized_number, body: normalized_body)
      when "move"
        client.move_to_column(account_id, normalized_number, column_id: normalized_column_id)
      when "close"
        client.close_card(account_id, normalized_number)
      when "reopen"
        client.reopen_card(account_id, normalized_number)
      end
    end

    private
      def kind_requirements
        case kind.to_s
        when "create"
          errors.add(:board_id, "is required to create a card") if normalized_board_id.blank?
          errors.add(:title, "is required to create a card") if normalized_title.blank?
        when "comment"
          errors.add(:number, "is required") if normalized_number.blank?
          errors.add(:body, "is required for a comment") if normalized_body.blank?
        when "move"
          errors.add(:number, "is required") if normalized_number.blank?
          errors.add(:column_id, "is required to move a card") if normalized_column_id.blank?
        when "close", "reopen"
          errors.add(:number, "is required") if normalized_number.blank?
        end
      end

      def text_length_within_payload
        if normalized_title && normalized_title.length > MAX_TITLE_CHARS
          errors.add(:title, "is too long (maximum is #{MAX_TITLE_CHARS} characters)")
        end
        if normalized_description && normalized_description.length > MAX_BODY_CHARS
          errors.add(:description, "is too long (maximum is #{MAX_BODY_CHARS} characters)")
        end
        if normalized_body && normalized_body.length > MAX_BODY_CHARS
          errors.add(:body, "is too long (maximum is #{MAX_BODY_CHARS} characters)")
        end
      end
  end
end
