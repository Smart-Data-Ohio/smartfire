module Message::Pagination
  extend ActiveSupport::Concern

  PAGE_SIZE = 40

  included do
    scope :last_page, -> { ordered.last(PAGE_SIZE) }
    scope :first_page, -> { ordered.first(PAGE_SIZE) }

    # Tuple comparison on (created_at, id): a strict created_at
    # comparison skips same-timestamp messages at page edges, and the
    # qualified columns keep joined scopes (search results join rooms
    # and memberships) from raising "ambiguous column name".
    scope :before, ->(message) { where("(messages.created_at, messages.id) < (?, ?)", message.created_at, message.id) }
    scope :after, ->(message) { where("(messages.created_at, messages.id) > (?, ?)", message.created_at, message.id) }

    scope :page_before, ->(message) { before(message).last_page }
    scope :page_after, ->(message) { after(message).first_page }

    scope :page_created_since, ->(time) { where("created_at > ?", time).first_page }
    scope :page_updated_since, ->(time) { where("updated_at > ?", time).last_page }
  end

  class_methods do
    def page_around(message)
      page_before(message) + [ message ] + page_after(message)
    end

    def paged?
      count > PAGE_SIZE
    end
  end
end
