use sqlx::{Executor, PgPool};
use tracing::field::display;
use tracing::Span;

use crate::domain::EmailAddress;
use crate::email_client::EmailClient;
use crate::routes::NewsletterIssueId;
use crate::utils::PgTransaction;

/// Newsletter issue
struct NewsletterIssue {
    title: String,
    content_html: String,
    content_text: String,
}

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

        match EmailAddress::parse(email.clone()) {
            // Valid email address: try to send the newsletter issue
            // TODO: implement a retry in case of a transient email delivery error
            Ok(email) => {
                let issue = get_issue(db_pool, issue_id).await?;
                if let Err(e) = email_client
                    .send_email(
                        &email,
                        &issue.title,
                        &issue.content_html,
                        &issue.content_text,
                    )
                    .await
                {
                    tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Failed to deliver issue to confirmed subscriber {}", email
                    );
                }
            }

            // Invalid email address: skip this particular subscriber
            Err(e) => {
                tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Skipping a confirmed subscriber because their stored contact details are invalid"
                );
            }
        }

        // Remove the task from the queue
        delete_task(transaction, issue_id, &email).await?;
    }
    Ok(())
}

/// Fetch a task from the newsletter issue delivery queue
#[tracing::instrument(skip_all)]
async fn dequeue_task(
    db_pool: &PgPool,
) -> anyhow::Result<Option<(PgTransaction, NewsletterIssueId, String)>> {
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
            r.subscriber_email,
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
    email: &str,
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
            email
        ))
        .await?;
    transaction.commit().await?;
    Ok(())
}

/// Fetch the newsletter content
#[tracing::instrument(skip_all)]
async fn get_issue(
    db_pool: &PgPool,
    issue_id: NewsletterIssueId,
) -> anyhow::Result<NewsletterIssue> {
    // Get the newsletter content associated with the provided `newsletter_issue_id`
    let issue = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, content_html, content_text
        FROM newsletter_issues
        WHERE newsletter_issue_id = $1
        "#,
        *issue_id
    )
    .fetch_one(db_pool)
    .await?;

    Ok(issue)
}
