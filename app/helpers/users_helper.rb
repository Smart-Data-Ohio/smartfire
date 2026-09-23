module UsersHelper
  # Data attributes turning any avatar or display name into a profile-card
  # trigger. Links and buttons activate through their native click (mouse or
  # Enter); non-interactive elements need keyboard: true plus tabindex and
  # role="button" so Enter and Space reach the card too.
  def profile_card_trigger(user, keyboard: false)
    actions = keyboard ? "click->profile-card#open keydown->profile-card#open" : "click->profile-card#open"
    { action: actions, profile_card_url: user_card_path(user) }
  end

  # Template for Stimulus-rendered rows, with USER_ID replaced client-side.
  def profile_card_url_template
    user_card_path("USER_ID")
  end

  def button_to_direct_room_with(user)
    button_to rooms_directs_path(user_ids: [ user.id ]), class: "btn btn--primary full-width txt--large",
        aria: { label: "Message #{user.name}" } do
      image_tag("messages.svg", aria: { hidden: "true" })
    end
  end
end
