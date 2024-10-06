use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::error::InternalError;
use actix_web::middleware::Next;
use actix_web::FromRequest;

use crate::session_state::TypedSession;
use crate::utils::{err500, see_other};

/// Reject users that are not logged in
#[allow(clippy::future_not_send)]
pub async fn reject_unauth_users(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> actix_web::Result<ServiceResponse<impl MessageBody>> {
    // Retrieve session
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    // Check if the session state contains a `user_id`
    if session.get_user_id().map_err(err500)?.is_some() {
        next.call(req).await
    } else {
        let response = see_other("/login");
        let e = anyhow::anyhow!("The user is not logged in");
        Err(InternalError::from_response(e, response).into())
    }
}
