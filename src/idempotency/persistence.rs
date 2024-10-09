use sqlx::PgPool;

use crate::authentication::UserId;
use crate::idempotency::IdempotencyKey;

pub async fn get_saved_response(
    db_pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: UserId,
) {
    todo!()
}
