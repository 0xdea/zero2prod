use std::time;

use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool};
use tracing::field::display;
use tracing::Span;

use crate::configuration::Settings;
use crate::domain::EmailAddress;
use crate::email_client::EmailClient;
use crate::routes::NewsletterIssueId;
use crate::utils::PgTransaction;

/// Delivery worker
pub struct DeliveryWorker {
    db_pool: PgPool,
    email_client: EmailClient,
}

impl DeliveryWorker {
    /// Build a worker based on settings
    pub fn build(config: Settings) -> anyhow::Result<Self> {
        // Connect to the database
        let db_pool = PgPoolOptions::new()
            .acquire_timeout(time::Duration::from_secs(2))
            .connect_lazy_with(config.database.db_options());

        // Build the worker
        Self::build_with_db_pool(config, &db_pool)
    }

    /// Build a worker based on settings and database pool
    pub fn build_with_db_pool(config: Settings, db_pool: &PgPool) -> anyhow::Result<Self> {
        // Build the email client
        let email_client = config.email_client.client();

        Ok(Self {
            db_pool: db_pool.clone(),
            email_client,
        })
    }

    /// Run the newsletter issue delivery worker until it is stopped
    pub async fn run_until_stopped(self) -> anyhow::Result<()> {
        worker_loop(self.db_pool, self.email_client).await
    }
}

/// Execution result
pub enum ExecutionResult {
    TaskCompleted,
    EmptyQueue,
}

/// Issue delivery worker loop
// TODO: refine the implementation to distinguish between transient and fatal failures (e.g., invalid subscriber email)
// TODO: improve the delay strategy by introducing exponential backoff with jitter
async fn worker_loop(db_pool: PgPool, email_client: EmailClient) -> anyhow::Result<()> {
    loop {
        match try_execute_task(&db_pool, &email_client).await {
            Err(_) => {
                tokio::time::sleep(time::Duration::from_secs(1)).await;
            }
            Ok(ExecutionResult::EmptyQueue) => {
                tokio::time::sleep(time::Duration::from_secs(10)).await;
            }
            Ok(ExecutionResult::TaskCompleted) => {}
        }
    }
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
pub async fn try_execute_task(
    db_pool: &PgPool,
    email_client: &EmailClient,
) -> anyhow::Result<ExecutionResult> {
    // Fetch a task from the queue, with an early return if the queue is empty
    let task = dequeue_task(db_pool).await?;
    if task.is_none() {
        return Ok(ExecutionResult::EmptyQueue);
    }

    // Process a task in the newsletter issue delivery queue
    let (transaction, issue_id, email) = task.unwrap();
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

    // Remove the task from the queue and return success
    delete_task(transaction, issue_id, &email).await?;
    Ok(ExecutionResult::TaskCompleted)
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

/// Newsletter issue
struct NewsletterIssue {
    title: String,
    content_html: String,
    content_text: String,
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
