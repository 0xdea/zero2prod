use actix_web::body::to_bytes;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use sqlx::{Executor, PgPool};

use crate::authentication::UserId;
use crate::idempotency::IdempotencyKey;
use crate::utils::PgTransaction;

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
            response_status_code AS "response_status_code!",
            response_headers AS "response_headers!: Vec<HeaderPairRecord>",
            response_body AS "response_body!"
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

/// Next action
#[allow(clippy::large_enum_variant)]
pub enum NextAction {
    StartProcessing(PgTransaction),
    ReturnSavedResponse(HttpResponse),
}

/// Try processing the request
pub async fn try_processing(
    db_pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: UserId,
) -> anyhow::Result<NextAction> {
    // Save the initial request fields to the database
    let mut transaction = db_pool.begin().await?;
    let n_inserted_rows = transaction
        .execute(sqlx::query!(
            r#"
            INSERT INTO idempotency (
                user_id,
                idempotency_key,
                created_at
            )
            VALUES ($1, $2, now())
            ON CONFLICT DO NOTHING
            "#,
            *user_id,
            idempotency_key.as_ref()
        ))
        .await?
        .rows_affected();

    // If the insert was successful, start processing the request
    if n_inserted_rows > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        // Otherwise, return the saved response
        let response = get_saved_response(db_pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Saved response not found"))?;
        Ok(NextAction::ReturnSavedResponse(response))
    }
}

/// Save response to the database
#[allow(clippy::future_not_send)]
pub async fn save_response(
    mut transaction: PgTransaction,
    idempotency_key: &IdempotencyKey,
    user_id: UserId,
    http_response: HttpResponse,
) -> anyhow::Result<HttpResponse> {
    // Get ownership of the body and buffer it in memory
    let (head, body) = http_response.into_parts();
    let body = to_bytes(body).await.map_err(|e| anyhow::anyhow!("{e}"))?;

    // Process the body
    let status_code = head.status();
    let headers = {
        let mut h = Vec::with_capacity(head.headers().len());
        for (name, value) in head.headers() {
            let name = name.as_str().to_string();
            let value = value.as_bytes().to_vec();
            h.push(HeaderPairRecord { name, value });
        }
        h
    };

    // Save the rest of the response to the database (query is not checked because we're using a custom type)
    #[allow(clippy::cast_possible_wrap)]
    transaction
        .execute(sqlx::query_unchecked!(
            r#"
        UPDATE idempotency
        SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1 AND
            idempotency_key = $2
        "#,
            *user_id,
            idempotency_key.as_ref(),
            status_code.as_u16() as i16,
            headers,
            body.as_ref(),
        ))
        .await?;
    transaction.commit().await?;

    // Re-assemble and return the response
    Ok(head.set_body(body).map_into_boxed_body())
}
