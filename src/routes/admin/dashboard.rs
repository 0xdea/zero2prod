use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::authentication::UserId;
use crate::utils::{e500_internal_server_error, get_username};

// TODO: Implement a login-protected admin functionality to invite other admins/collaborators

/// Admin dashboard handler
pub async fn dashboard(
    db_pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> actix_web::Result<HttpResponse> {
    // Validate session and retrieve associated `user_id` and `username`
    let user_id = user_id.into_inner();
    let username = get_username(user_id, &db_pool)
        .await
        .map_err(e500_internal_server_error)?;

    // Display admin dashboard containing the retrieved `username`
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(include_str!("dashboard.html"), username)))
}
