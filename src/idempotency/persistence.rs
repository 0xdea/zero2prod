use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use sqlx::PgPool;

use crate::authentication::UserId;
use crate::idempotency::IdempotencyKey;

/// Header pair record
#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

/// Get saved response
pub async fn get_saved_response(
    db_pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: UserId,
) -> anyhow::Result<Option<HttpResponse>> {
    // Get saved response from the database
    let saved_response = sqlx::query!(
        r#"
        SELECT
            response_status_code,
            response_headers as "response_headers: Vec<HeaderPairRecord>",
            response_body
        FROM idempotency
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
        *user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(db_pool)
    .await?;

    // Map the retrieved data (if any) into a proper `HttpResponse`
    if let Some(r) = saved_response {
        let status_code = StatusCode::from_u16(r.response_status_code.try_into()?)?;
        let mut response = HttpResponse::build(status_code);
        for HeaderPairRecord { name, value } in r.response_headers {
            response.append_header((name, value));
        }
        Ok(Some(response.body(r.response_body)))
    } else {
        Ok(None)
    }
}
