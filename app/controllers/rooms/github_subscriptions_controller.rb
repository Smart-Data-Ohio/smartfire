class Rooms::GithubSubscriptionsController < ApplicationController
  include RoomScoped

  before_action :ensure_subscribable_room
  before_action :ensure_can_administer_room

  # Subscribing posts the repository's PR events to every room member, so
  # the subscriber must prove access first: their own linked GitHub account
  # has to read the repository (GET /repos/{owner}/{repo}). Administrators
  # may skip the check; such subscriptions are unverified and never post
  # private PR titles (see Github::Notifier).
  def create
    subscription = @room.github_repository_subscriptions.build(subscription_attributes.merge(created_by: Current.user))

    if subscription.valid?
      subscription.reader_verified = subscriber_can_read?(subscription)

      unless subscription.reader_verified? || administrator_override?
        return redirect_to edit_room_path, alert: unverified_alert(subscription)
      end
    end

    if subscription.save
      redirect_to edit_room_path, notice: "Subscribed to #{subscription.full_name}."
    else
      redirect_to edit_room_path, alert: "Could not subscribe: #{subscription.errors.full_messages.to_sentence}."
    end
  rescue ActiveRecord::RecordNotUnique
    redirect_to edit_room_path, alert: "Could not subscribe: that repository is already subscribed in this room."
  end

  def update
    subscription = @room.github_repository_subscriptions.find(params[:id])

    if subscription.update(events: events_param)
      redirect_to edit_room_path, notice: "Subscription to #{subscription.full_name} updated."
    else
      redirect_to edit_room_path, alert: "Could not update: #{subscription.errors.full_messages.to_sentence}."
    end
  end

  def destroy
    subscription = @room.github_repository_subscriptions.find(params[:id])
    subscription.destroy!

    redirect_to edit_room_path, notice: "Unsubscribed from #{subscription.full_name}."
  end

  private
    # RoomScoped#find_by! answers 404 for non-members; direct rooms are never
    # subscribable, so they 404 as well.
    def ensure_subscribable_room
      head :not_found if @room.direct?
    end

    def ensure_can_administer_room
      head :forbidden unless Current.user.can_administer?(@room)
    end

    def subscription_attributes
      owner, repo = parse_full_name(subscription_params[:full_name])

      { owner: owner, repo: repo, events: events_param || Github::RepositorySubscription::DEFAULT_EVENTS.dup }
    end

    def subscription_params
      params.require(:github_repository_subscription).permit(:full_name, :skip_access_check, events: [])
    end

    def subscriber_can_read?(subscription)
      Current.user.github_connected_account&.can_read_repository?(subscription.owner, subscription.repo) || false
    end

    def administrator_override?
      Current.user.administrator? && subscription_params[:skip_access_check] == "1"
    end

    def unverified_alert(subscription)
      account = Current.user.github_connected_account
      reason = if account&.usable?
        "your linked GitHub account could not confirm it can read #{subscription.full_name}"
      else
        "link your GitHub account on your profile so it can confirm you can read #{subscription.full_name}"
      end

      "Could not subscribe: #{reason}.#{" Administrators may subscribe without verifying access." if Current.user.administrator?}"
    end

    def events_param
      events = subscription_params[:events]
      events&.reject(&:blank?)&.uniq
    end

    def parse_full_name(full_name)
      owner, repo = full_name.to_s.strip.split("/", 2).map { |part| part.to_s.strip }
      [ owner.presence, repo.presence ]
    end

    def edit_room_path
      @room.open? ? edit_rooms_open_path(@room) : edit_rooms_closed_path(@room)
    end
end
