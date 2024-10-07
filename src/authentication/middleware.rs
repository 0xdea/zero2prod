use std::fmt;
use std::ops::Deref;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::error::InternalError;
use actix_web::middleware::Next;
use actix_web::{FromRequest, HttpMessage};
use uuid::Uuid;

use crate::session_state::TypedSession;
use crate::utils::{err500, see_other};

/// User identifier
#[derive(Copy, Clone, Debug)]
pub struct UserId(Uuid);

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Reject users that are not logged in
#[allow(clippy::future_not_send)]
pub async fn reject_logged_out_users(
    mut req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> actix_web::Result<ServiceResponse<impl MessageBody>> {
    // Retrieve session state
    let session = {
        let (http_request, payload) = req.parts_mut();
        TypedSession::from_request(http_request, payload).await
    }?;

    // Check if the session state contains a `user_id`, otherwise return error
    if let Some(user_id) = session.get_user_id().map_err(err500)? {
        req.extensions_mut().insert(UserId(user_id));
        next.call(req).await
    } else {
        let response = see_other("/login");
        let e = anyhow::anyhow!("The user is not logged in");
        Err(InternalError::from_response(e, response).into())
    }
}
