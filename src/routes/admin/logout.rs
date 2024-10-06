use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

use crate::session_state::TypedSession;
use crate::utils::{err500, see_other};

/// Logout handler
pub async fn logout(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    if let Some(_user_id) = session.get_user_id().map_err(err500)? {
        session.logout();
        FlashMessage::info("You have successfully logged out").send();
    }

    Ok(see_other("/login"))
}
