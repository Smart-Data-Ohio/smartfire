pin "application"

pin "@hotwired/stimulus", to: "stimulus.min.js"
pin "@hotwired/stimulus-loading", to: "stimulus-loading.js"
pin "@hotwired/turbo-rails", to: "turbo.js"
pin "@rails/actioncable", to: "actioncable.esm.js"
pin "@rails/request.js", to: "@rails--request.js" # @0.0.8
pin "code-highlighter-worker", to: "code-highlighter-worker.js", preload: false
pin "livekit-client", to: "livekit-client.js", preload: false
pin "noise-suppressor", to: "noise-suppressor.js", preload: false

pin_all_from "app/javascript/initializers", under: "initializers"
pin_all_from "app/javascript/lib", under: "lib"
pin_all_from "app/javascript/channels", under: "channels"
# Controllers stay resolvable for lazyLoadControllersFrom but are not
# downloaded up front; pages preload only their first-paint set (see
# RoomsHelper::FIRST_PAINT_CONTROLLERS).
pin_all_from "app/javascript/controllers", under: "controllers", preload: false
pin_all_from "app/javascript/helpers", under: "helpers"
pin_all_from "app/javascript/models", under: "models"
