use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

/// Web query parameters
#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

/// Confirmation handler
// TODO: What happens if a user clicks on a confirmation link twice?
#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, db_pool))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    db_pool: web::Data<PgPool>,
) -> HttpResponse {
    // Get subscriber id from subscription token
    let Ok(id) = get_subscriber_id_from_token(&parameters.subscription_token, &db_pool).await
    else {
        // TODO: StatusCode?
        return HttpResponse::InternalServerError().finish();
    };

    // Confirm subscriber if token is valid
    match id {
        // Non-existing token
        // TODO: StatusCode?
        None => HttpResponse::Unauthorized().finish(),

        // Valid token
        Some(subscriber_id) => confirm_subscriber(subscriber_id, &db_pool)
            .await
            .map_or_else(
                // TODO: StatusCode?
                |_| HttpResponse::InternalServerError().finish(),
                |_| HttpResponse::Ok().finish(),
            ),
    }
}

/// Get subscriber id from subscription token
// TODO: Add validation on the incoming token, we are currently passing the raw user input straight into a query
#[tracing::instrument(
    name = "Getting subscriber id from subscription token",
    skip(subscription_token, db_pool)
)]
pub async fn get_subscriber_id_from_token(
    subscription_token: &str,
    db_pool: &PgPool,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token,
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}

/// Mark subscriber as confirmed
#[tracing::instrument(name = "Marking subscriber as confirmed", skip(subscriber_id, db_pool))]
pub async fn confirm_subscriber(subscriber_id: Uuid, db_pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
