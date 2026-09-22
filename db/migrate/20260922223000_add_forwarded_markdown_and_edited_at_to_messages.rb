class AddForwardedMarkdownAndEditedAtToMessages < ActiveRecord::Migration[8.2]
  def change
    # True when a forward's snapshot body is Markdown-rendered HTML, so it
    # renders through the Markdown presentation and sanitizer instead of
    # the legacy plain-text path. Legacy forwards keep rendering as now.
    add_column :messages, :forwarded_markdown, :boolean, default: false, null: false
    # Stamped by the edit endpoints only, so "(edited)" never flips for
    # reaction touches, reply tombstones or card fetches.
    add_column :messages, :edited_at, :datetime
  end
end
