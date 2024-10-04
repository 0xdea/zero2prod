use std::future::{ready, Ready};

use actix_session::{Session, SessionExt, SessionGetError, SessionInsertError};
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use uuid::Uuid;

/// Session type
pub struct TypedSession(Session);

impl FromRequest for TypedSession {
    // Return the same error returned by the implementation of `FromRequest` for `Session`
    type Error = <Session as FromRequest>::Error;

    // Wrap `TypedSession` into `Ready` to convert it into a `Future`
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(Self(req.get_session())))
    }
}

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    /// Renew the session key
    pub fn renew(&self) {
        self.0.renew();
    }

    /// Insert `user_id` into session
    pub fn insert_user_id(&self, user_id: Uuid) -> Result<(), SessionInsertError> {
        self.0.insert(Self::USER_ID_KEY, user_id)
    }

    ///  Get `user_id` from session
    pub fn get_user_id(&self) -> Result<Option<Uuid>, SessionGetError> {
        self.0.get(Self::USER_ID_KEY)
    }
}
