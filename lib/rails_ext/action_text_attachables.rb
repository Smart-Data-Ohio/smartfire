ActiveSupport.on_load(:action_text_content) do
  # Content#attachables resolves through Attachable.from_node directly,
  # bypassing Attachment.from_node, so the preloaded lookup hooks both.
  module ActionText::Attachable
    class << self
      def from_node_with_preloaded_user(node)
        Message::MentionPreloader.preloaded_user_for(node) || from_node_without_preloaded_user(node)
      end

      alias_method :from_node_without_preloaded_user, :from_node
      alias_method :from_node, :from_node_with_preloaded_user
    end
  end

  class ActionText::Attachment
    class << self
      def from_node(node, attachable = nil)
        new(node, attachable || ActionText::Attachment::OpengraphEmbed.from_node(node) || Message::MentionPreloader.preloaded_user_for(node) || attachable_from_possibly_expired_sgid(node["sgid"]) || ActionText::Attachable.from_node(node))
      end

      private
        # Our @mentions use ActionText attachments, which are signed. If someone rotates SECRET_KEY_BASE, the existing attachments become invalid.
        # This allows ignoring invalid signatures for User attachments in ActionText.
        ATTACHABLES_PERMITTED_WITH_INVALID_SIGNATURES = %w[ User ]

        def attachable_from_possibly_expired_sgid(sgid)
          if gid_uri = Message::MentionPreloader.gid_uri_for_sgid(sgid)
            if model = GlobalID.find(gid_uri)
              model.model_name.to_s.in?(ATTACHABLES_PERMITTED_WITH_INVALID_SIGNATURES) ? model : nil
            end
          end
        rescue ActiveRecord::RecordNotFound
          nil
        end
    end
  end
end
