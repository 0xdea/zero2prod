use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

use crate::session_state::TypedSession;
use crate::utils::see_other;

/// Logout handler
#[allow(clippy::future_not_send)]
pub async fn logout(session: TypedSession) -> actix_web::Result<HttpResponse> {
    // Perform logout, redirect to login form, and return info in flash message
    session.logout();
    FlashMessage::info("You have successfully logged out").send();
    Ok(see_other("/login"))
}
