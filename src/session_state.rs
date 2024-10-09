use std::future::{ready, Ready};

use actix_session::{Session, SessionExt, SessionGetError, SessionInsertError};
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};

use crate::authentication::UserId;

/// Session type
pub struct TypedSession(Session);

impl FromRequest for TypedSession {
    type Error = <Session as FromRequest>::Error;
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
    pub fn insert_user_id(&self, user_id: UserId) -> Result<(), SessionInsertError> {
        self.0.insert(Self::USER_ID_KEY, user_id)
    }

    ///  Get `user_id` from session
    pub fn get_user_id(&self) -> Result<Option<UserId>, SessionGetError> {
        self.0.get(Self::USER_ID_KEY)
    }

    /// Purge session data to logout
    pub fn logout(self) {
        self.0.purge();
    }
}
