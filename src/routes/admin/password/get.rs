use actix_web::http::header::ContentType;
use actix_web::HttpResponse;

use crate::session_state::TypedSession;
use crate::utils::{err500, see_other};

/// Admin password GET handler
#[allow(clippy::future_not_send)]
pub async fn password_form(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    if session.get_user_id().map_err(err500)?.is_none() {
        return Ok(see_other("/login"));
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(include_str!("password_form.html")))
}
