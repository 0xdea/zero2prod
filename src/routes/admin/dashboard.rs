use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::session_state::TypedSession;
use crate::utils::{err500, get_username, see_other};

// TODO: implement a login-protected admin functionality to invite more admins/collaborators

/// Admin dashboard handler
#[allow(clippy::future_not_send)]
pub async fn dashboard(
    session: TypedSession,
    db_pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    // Validate session and retrieve associated username
    let username = if let Some(user_id) = session.get_user_id().map_err(err500)? {
        get_username(user_id, &db_pool).await.map_err(err500)?
    } else {
        return Ok(see_other("/login"));
    };

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(include_str!("dashboard.html"), username)))
}
