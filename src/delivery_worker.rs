use sqlx::{Executor, PgPool};
use tracing::field::display;
use tracing::Span;

use crate::domain::EmailAddress;
use crate::email_client::EmailClient;
use crate::routes::NewsletterIssueId;
use crate::utils::PgTransaction;

/// Try executing a task in the newsletter issue delivery queue
#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty
    ),
    err
)]
async fn try_execute_task(db_pool: &PgPool, email_client: &EmailClient) -> anyhow::Result<()> {
    // Process a task in the newsletter issue delivery queue
    if let Some((transaction, issue_id, email)) = dequeue_task(db_pool).await? {
        Span::current()
            .record("newsletter_issue_id", display(issue_id))
            .record("subscriber_email", display(&email));
        // TODO: actually send email
        delete_task(transaction, issue_id, &email).await?;
    }
    Ok(())
}

/// Fetch a task from the newsletter issue delivery queue
#[tracing::instrument(skip_all)]
async fn dequeue_task(
    db_pool: &PgPool,
) -> anyhow::Result<Option<(PgTransaction, NewsletterIssueId, EmailAddress)>> {
    // Query the database to fetch a task
    let mut transaction = db_pool.begin().await?;
    let r = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, subscriber_email
        FROM issue_delivery_queue
        FOR UPDATE
        SKIP LOCKED
        LIMIT 1
        "#
    )
    .fetch_optional(&mut *transaction)
    .await?;

    // Return the task data
    if let Some(r) = r {
        Ok(Some((
            transaction,
            NewsletterIssueId::new(r.newsletter_issue_id),
            EmailAddress::parse(r.subscriber_email).unwrap(),
        )))
    } else {
        Ok(None)
    }
}

/// Remove a task from the newsletter issue delivery queue
#[tracing::instrument(skip_all)]
async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: NewsletterIssueId,
    email_address: &EmailAddress,
) -> anyhow::Result<()> {
    // Delete a task from the database
    transaction
        .execute(sqlx::query!(
            r#"
            DELETE FROM issue_delivery_queue
            WHERE
                newsletter_issue_id = $1 AND
                subscriber_email = $2
            "#,
            *issue_id,
            email_address.as_ref()
        ))
        .await?;
    transaction.commit().await?;
    Ok(())
}
