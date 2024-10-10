use actix_web::body::to_bytes;
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

/// Get saved response from the database
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

/// Save response to the database
pub async fn save_response(
    db_pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: UserId,
    http_response: HttpResponse,
) -> anyhow::Result<HttpResponse> {
    // Get ownership of the body and buffer it in memory
    let (head, body) = http_response.into_parts();
    let body = to_bytes(body).await.map_err(|e| anyhow::anyhow!("{e}"))?;

    // Process the body
    let status_code = head.status().as_u16() as i16;
    let headers = {
        let mut h = Vec::with_capacity(head.headers().len());
        for (name, value) in head.headers() {
            let name = name.as_str().to_string();
            let value = value.as_bytes().to_vec();
            h.push(HeaderPairRecord { name, value });
        }
        h
    };

    // TODO: SQL query

    // Re-assemble and return the response
    Ok(head.set_body(body).map_into_boxed_body())
}
