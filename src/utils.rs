use std::fmt;

use actix_web::http::header::LOCATION;
use actix_web::HttpResponse;

/// Return an opaque Error 500 while preserving the error's cause for logging purposes
pub fn err500<T>(err: T) -> actix_web::Error
where
    T: fmt::Debug + fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(err)
}

/// Return an Error 303 and redirect to the specified location
pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}
