Rails.application.routes.draw do
  root "welcome#show"

  namespace :internal do
    post "huddle/authorize", to: "huddle#authorize"
    get "huddle/grants/:id", to: "huddle#show"
    post "huddle/grants/:id/left", to: "huddle#left"
  end

  resource :first_run

  resource :session do
    scope module: "sessions" do
      resources :transfers, only: %i[ show update ]
    end
  end

  post "csp_reports", to: "content_security_policy_reports#create", as: :content_security_policy_reports

  post "session/google", to: "sessions/google#create", as: :session_google
  get "session/google/callback", to: "sessions/google#callback", as: :session_google_callback
  post "user/profile/google_sign_in_link", to: "users/google_sign_in_links#create", as: :user_google_sign_in_link

  resource :account do
    scope module: "accounts" do
      resources :users do
        scope module: "users" do
          resource :google_link, only: %i[ create destroy ]
        end
      end

      resources :bots do
        post :kill_switch, on: :member
        scope module: "bots" do
          resource :key, only: :update
          resources :credentials, only: %i[ index create destroy ]
          resources :grants, only: %i[ index create destroy ]
          resource :github_connection, only: %i[ create destroy ]
          resource :webhook_secret, only: :create
        end
      end

      resources :icons, only: %i[ index create destroy ]

      resource :join_code, only: :create
      resource :logo, only: %i[ show destroy ]
      resource :custom_styles, only: %i[ edit update ]
      resource :audit_log, only: :show, controller: "audit_logs"
      resource :integrations_health, only: :show, controller: "integrations_health"
    end
  end

  direct :fresh_account_logo do |options|
    route_for :account_logo, v: Current.account&.updated_at&.to_fs(:number), size: options[:size]
  end

  get "join/:join_code", to: "users#new", as: :join
  post "join/:join_code", to: "users#create"

  resources :qr_code, only: :show

  get "users/:id/card", to: "users/cards#show", as: :user_card

  resources :users, only: %i[ index show ] do
    get :huddle_presence, on: :collection, to: "users/huddle_presence#show"
    get :presence, on: :collection, to: "users/presences#show"

    scope module: "users" do
      resource :avatar, only: %i[ show destroy ]
      resource :ban, only: %i[ create destroy ]
      resource :dnd_allowance, only: %i[ create destroy ]

      scope defaults: { user_id: "me" } do
        resource :sidebar, only: :show
        resource :profile
        resource :status, only: :update, controller: "statuses"
        resource :notification_settings, only: :update
        resource :time_zone, only: :update, controller: "time_zones"
        resources :push_subscriptions do
          scope module: "push_subscriptions" do
            resources :test_notifications, only: :create
          end
        end
      end
    end
  end

  namespace :autocompletable do
    resources :users, only: :index
    resources :icons, only: :index
    resources :slash_commands, only: :index
  end

  get "icons/:name", to: "workspace_icons#show", as: :workspace_icon

  get "agents", to: "agents/directory#index"
  get "agents/me", to: "agents#me", defaults: { format: :json }
  patch "agents/me", to: "agents#update", defaults: { format: :json }
  get "agents/events", to: "agents/events#index", defaults: { format: :json }
  post "agents/events/:id/ack", to: "agents/events#ack", defaults: { format: :json }, as: :ack_agents_event
  get "agents/:id/events", to: "agents/events#ledger", as: :agent_events
  get "agents/approvals", to: "agents/approvals#index", defaults: { format: :json }
  post "agents/approvals", to: "agents/approvals#create", defaults: { format: :json }
  get "agents/approvals/:id", to: "agents/approvals#show", defaults: { format: :json }
  delete "agents/approvals/:id", to: "agents/approvals#destroy", defaults: { format: :json }
  get "agents/:id/approvals", to: "agents/approvals#for_agent", as: :agent_approvals
  patch "agent_approvals/:id", to: "agent_approvals#update", as: :agent_approval
  get "agents/context", to: "agents/contexts#show", defaults: { format: :json }
  post "agents/dms", to: "agents/dms#create", defaults: { format: :json }
  post "agents/mcp", to: "agents/mcp#create", defaults: { format: :json }
  get "agents/mcp", to: "agents/mcp#method_not_allowed", defaults: { format: :json }
  delete "agents/mcp", to: "agents/mcp#method_not_allowed", defaults: { format: :json }
  get "agents/work", to: "agents/work#index", defaults: { format: :json }, as: :agents_work
  get "agents/work/:id", to: "agents/work#show", defaults: { format: :json }, as: :agents_work_thread
  patch "agents/work/:id", to: "agents/work#update", defaults: { format: :json }
  put "agents/work/:id/result", to: "agents/work#result", defaults: { format: :json }
  post "agents/work/:id/handoff", to: "agents/work#handoff", defaults: { format: :json }
  post "rooms/:room_id/agents/messages", to: "agents/messages#create", defaults: { format: :json }, as: :room_agent_messages
  post "rooms/:room_id/agents/streaming_messages", to: "agents/streaming_messages#create", defaults: { format: :json }, as: :room_agent_streaming_messages
  patch "agents/streaming_messages/:id", to: "agents/streaming_messages#update", defaults: { format: :json }, as: :agents_streaming_message
  post "agents/streaming_messages/:id/finalize", to: "agents/streaming_messages#finalize", defaults: { format: :json }, as: :finalize_agents_streaming_message
  post "agents/steps", to: "agents/steps#create", defaults: { format: :json }, as: :agents_steps
  patch "agents/steps/:id", to: "agents/steps#update", defaults: { format: :json }, as: :agents_step
  post "agents/messages/:id/pin", to: "agents/pins#create", defaults: { format: :json }, as: :agents_message_pin
  delete "agents/messages/:id/pin", to: "agents/pins#destroy", defaults: { format: :json }
  post "rooms/:room_id/agents/slash_commands", to: "agents/slash_commands#create", defaults: { format: :json }
  delete "rooms/:room_id/agents/slash_commands/:name", to: "agents/slash_commands#destroy", defaults: { format: :json }
  post "rooms/:room_id/agents/polls", to: "agents/polls#create", defaults: { format: :json }
  get "rooms/:room_id/agents/polls/:id", to: "agents/polls#show", defaults: { format: :json }
  get "rooms/:room_id/agents/posts", to: "agents/posts#index", defaults: { format: :json }, as: :room_agent_posts
  post "rooms/:room_id/agents/posts", to: "agents/posts#create", defaults: { format: :json }
  post "rooms/:room_id/agents/github/pull_request_actions", to: "agents/github/pull_request_actions#create",
    defaults: { format: :json }, as: :room_agent_github_pull_request_actions
  get "agents/fizzy/boards", to: "agents/fizzy/boards#index", defaults: { format: :json }
  get "agents/fizzy/boards/:id", to: "agents/fizzy/boards#show", defaults: { format: :json }
  get "agents/fizzy/cards/search", to: "agents/fizzy/cards#search", defaults: { format: :json }
  get "agents/fizzy/cards/:account_id/:number", to: "agents/fizzy/cards#show", defaults: { format: :json }
  post "agents/fizzy/card_actions", to: "agents/fizzy/card_actions#create", defaults: { format: :json }

  direct :fresh_user_avatar do |user, options|
    route_for :user_avatar, user.avatar_token, v: user.updated_at.to_fs(:number)
  end

  resources :rooms do
    resources :messages do
      post :preview, on: :collection
      get :actions, on: :member
      get :forward_source, on: :member, controller: "message_forward_sources"
      resource :embed_suppression, controller: "message_embed_suppressions", only: :create
      resources :forwards, controller: "message_forwards", only: :create
      get "forwards/destinations", to: "message_forwards#destinations", as: :forward_destinations
      resources :fizzy_cards, only: %i[ new create ], controller: "rooms/fizzy/message_cards"
    end

    resources :threads, controller: "channel_threads", only: %i[ index show new create update destroy ] do
      get :content, on: :member
      resources :messages, controller: "channel_thread_messages", only: %i[ index show create update destroy ] do
        get :actions, on: :member
        get :forward_source, on: :member, controller: "message_forward_sources"
        resource :embed_suppression, controller: "message_embed_suppressions", only: :create
        resources :forwards, controller: "message_forwards", only: :create
        get "forwards/destinations", to: "message_forwards#destinations", as: :forward_destinations
        resources :fizzy_cards, only: %i[ new create ], controller: "rooms/fizzy/message_cards"
      end
      post :join, on: :member
      delete :leave, on: :member
      post :read, on: :member
      patch :read, on: :member
    end

    nested do
      scope path: ":bot_key", as: :bot, defaults: { format: :json } do
        resources :messages, controller: "messages/by_bots", only: %i[ index create update destroy ] do
          resources :boosts, controller: "messages/boosts/by_bots", only: %i[ create destroy ]
        end
      end
    end

    scope module: "rooms" do
      resources :members, only: :index
      resources :message_links, only: :show
      resources :files, only: :index
      resources :pins, only: :index
      resources :slash_commands, only: :create
      resources :polls, only: %i[ create show ] do
        post :vote, on: :member
      end
      resources :drive_recipients, only: :index do
        post :validate, on: :collection
      end
      resources :events, only: %i[ index show new create edit update ] do
        patch :cancel, on: :member
        resource :attendance, only: %i[ show update ], controller: "events/attendances"
      end
      namespace :stage do
        resource :hand, only: %i[ create destroy ], controller: "hands"
        resource :stream, only: %i[ create destroy ], controller: "streams"
        patch "roles/:membership_id", to: "roles#update", as: :role
      end
      resource :huddle, only: %i[ show create ] do
        get :participants
        post :leave
      end
      post "call_moderation/:membership_id/mute", to: "call_moderation#mute", as: :call_moderation_mute
      delete "call_moderation/:membership_id/mute", to: "call_moderation#unmute", as: :call_moderation_unmute
      post "call_moderation/:membership_id/disconnect", to: "call_moderation#disconnect", as: :call_moderation_disconnect
      resource :refresh, only: :show
      resource :settings, only: :show
      resource :involvement, only: %i[ show update ]
      resource :read, only: %i[ create destroy ], controller: "reads"
      resource :favorite, only: %i[ create destroy update ], controller: "favorites"
      resource :category_assignment, only: :update, controller: "categories"
      resources :github_subscriptions, only: %i[ create update destroy ]
      resource :inbound_email_address, only: :create, controller: "inbound_email_addresses"
    end

    namespace :github do
      resources :pull_request_threads, only: :create
      resources :pull_request_comments, only: :create
      resources :pull_request_reviews, only: :create
      resources :pull_request_review_requests, only: :create
      resources :pull_request_write_actions, only: :show
    end

    # Per-viewer card frame for private-repository PRs. Served from the
    # Rooms::Github namespace (room-membership scoped) rather than the
    # shared Github namespace above, because the response differs per
    # viewer while the surrounding message HTML is cached across viewers.
    get "github/pull_requests/:id/card", to: "rooms/github/pull_request_cards#show", as: :github_pull_request_card

    # Per-viewer card frame for Fizzy cards referenced in the room. Every
    # Fizzy card loads through here (there is no public fast path: card
    # content always depends on the viewer's own Fizzy token).
    get "fizzy/cards/:id/card", to: "rooms/fizzy/cards#show", as: :fizzy_card

    get "@:message_id", to: "rooms#show", as: :at_message
  end

  namespace :rooms do
    resources :opens
    resources :closeds
    resources :directs do
      post :add_members, on: :member
      delete :leave, on: :member
    end
    resources :voices
    resources :stages
    resources :boards
  end

  get "rooms/boards/:board_id/automations", to: "rooms/boards/automations#show", as: :board_automations
  post "rooms/boards/:board_id/automations/tag_assignments", to: "rooms/boards/automations#create_tag_assignment", as: :board_automation_tag_assignments
  delete "rooms/boards/:board_id/automations/tag_assignments/:id", to: "rooms/boards/automations#destroy_tag_assignment", as: :board_automation_tag_assignment
  patch "rooms/boards/:board_id/automations/sla_rules", to: "rooms/boards/automations#update_sla_rules", as: :board_automation_sla_rules

  resources :messages do
    resources :forwards, controller: "message_forwards", only: :create
    resource :embed_suppression, controller: "message_embed_suppressions", only: :create
    get :forward_source, on: :member, controller: "message_forward_sources"
    get "forwards/destinations", to: "message_forwards#destinations", as: :forward_destinations
    resource :pin, controller: "messages/pins", only: %i[ create destroy ]

    scope module: "messages" do
      resources :boosts
    end
  end

  resources :saved_items, path: "saved", only: %i[ index create update destroy ]

  resources :scheduled_messages, only: %i[ index update destroy ] do
    post :send_now, on: :member
  end
  post "rooms/:room_id/scheduled_messages", to: "scheduled_messages#create", as: :room_scheduled_messages

  resources :searches, only: %i[ index create ] do
    delete :clear, on: :collection
  end

  resource :switcher, only: :show
  resources :room_categories, only: %i[ index create update destroy ]

  resources :activity_items, path: "activity", only: :index do
    get :unread_count, on: :collection
    post :open, on: :member
    patch :read, on: :member
    patch :handled, on: :member
  end

  resources :work_threads, path: "work", only: :index

  get "threads/:thread_id/work/links", to: "threads/work/links#index", as: :thread_work_links
  post "threads/:thread_id/work/links", to: "threads/work/links#create"
  delete "threads/:thread_id/work/links/:id", to: "threads/work/links#destroy", as: :thread_work_link
  get "threads/:thread_id/work/handoff/new", to: "threads/work/handoffs#new", as: :new_thread_work_handoff
  post "threads/:thread_id/work/handoff", to: "threads/work/handoffs#create", as: :thread_work_handoff

  resource :unfurl_link, only: :create

  namespace :github do
    post "webhooks", to: "webhooks#create"
    resource :connection, only: %i[ create destroy ], controller: "connections"
    get "app/connect", to: "app_connections#connect", as: :app_connect
    get "app/callback", to: "app_connections#callback", as: :app_callback
  end

  namespace :fizzy do
    resource :connection, only: %i[ create destroy ], controller: "connections"
  end

  namespace :google do
    post "connect", to: "connections#connect"
    get "callback", to: "connections#callback"
    delete "connection", to: "connections#destroy"
    post "calendar/notifications", to: "calendar_notifications#create"
    get "drive/files", to: "drive_files#index", as: :drive_files
    get "drive/files/:id", to: "drive_files#show", as: :drive_file
  end

  get "/about", to: "public_pages#about", as: :about
  get "/privacy", to: "public_pages#privacy", as: :privacy
  get "/terms", to: "public_pages#terms", as: :terms

  get "webmanifest"    => "pwa#manifest"
  get "service-worker" => "pwa#service_worker"

  get "up" => "rails/health#show", as: :rails_health_check

  # Test-only fast sign-in for system tests (see TestSessionController).
  # Never loaded outside the test environment.
  if Rails.env.test?
    require_relative "../test/support/test_session_controller"
    get "test_session", to: "test_session#create", as: :sign_in_for_tests
  end
end
