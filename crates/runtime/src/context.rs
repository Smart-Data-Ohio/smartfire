use campfire_app::app::AppState;
use campfire_kit::{Ctx, Error};

thread_local! {
    static RENDER_TIME_ZONE: std::cell::RefCell<Option<campfire_presentation::time::Zone>> = const { std::cell::RefCell::new(None) };
}

/// `app/controllers/concerns/set_time_zone.rb` scopes the write and after-commit renderers.
/// This contains only Time.zone; broadcasts still have no Current.user or session.
pub struct TimeZoneGuard(Option<campfire_presentation::time::Zone>);
impl Drop for TimeZoneGuard {
    fn drop(&mut self) {
        RENDER_TIME_ZONE.with(|zone| zone.replace(self.0.take()));
    }
}
pub fn enter_time_zone(zone: campfire_presentation::time::Zone) -> TimeZoneGuard {
    TimeZoneGuard(RENDER_TIME_ZONE.with(|current| current.replace(Some(zone))))
}
pub fn renderer_time_zone() -> campfire_presentation::time::Zone {
    RENDER_TIME_ZONE.with(|zone| zone.borrow().clone().unwrap_or_else(campfire_presentation::time::Zone::utc))
}

/// config/initializers/default_url_options.rb: APP_URL configures jobs as well as mail.
/// ActionController's renderer supplies example.org only when no origin is configured.
pub fn default_renderer_base_url(app: &AppState) -> &str {
    app.config.mail.app_url.as_deref().unwrap_or("http://example.org")
}

/// `SetCurrentRequest#default_url_options` supplies the request's host, port and protocol to
/// the detached renderer, including a nonstandard port.
pub fn renderer_base_url(c: &Ctx) -> String {
    c.url_for("")
}

pub fn db_error(error: campfire_db::Error) -> Error {
    match error {
        campfire_db::Error::RecordNotFound(_) => Error::NotFound,
        other => Error::internal(other),
    }
}
