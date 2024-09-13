use std::fmt::{Display, Formatter};
use std::{error, fmt, iter};

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::{EmailAddress, NewSubscriber, SubscriberName};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;

/// Web form data
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let email = EmailAddress::parse(value.email)?;
        let name = SubscriberName::parse(value.name)?;
        Ok(Self { email, name })
    }
}

/// Subscription errors
#[derive(Debug)]
pub enum SubscribeError {
    ValidationError(String),
    DatabaseError(sqlx::Error),
    StoreTokenError(StoreTokenError),
    SendEmailError(reqwest::Error),
}

impl Display for SubscribeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to create a new subscriber")
    }
}

impl From<reqwest::Error> for SubscribeError {
    fn from(e: reqwest::Error) -> Self {
        Self::SendEmailError(e)
    }
}
impl From<sqlx::Error> for SubscribeError {
    fn from(e: sqlx::Error) -> Self {
        Self::DatabaseError(e)
    }
}
impl From<StoreTokenError> for SubscribeError {
    fn from(e: StoreTokenError) -> Self {
        Self::StoreTokenError(e)
    }
}
impl From<String> for SubscribeError {
    fn from(e: String) -> Self {
        Self::ValidationError(e)
    }
}

impl error::Error for SubscribeError {}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::DatabaseError(_)
            | SubscribeError::StoreTokenError(_)
            | SubscribeError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Subscriptions handler
#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, db_pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name= %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    // Parse subscriber data and begin database transaction
    let new_subscriber = form.0.try_into()?;
    let mut transaction = db_pool.begin().await?;

    // Insert subscriber, generate and store a subscription token
    let subscriber_id = insert_subscriber(&new_subscriber, &mut transaction).await?;
    let subscription_token = generate_subscription_token();
    store_token(subscriber_id, &subscription_token, &mut transaction).await?;

    // End database transaction and send confirmation email with subscription token
    transaction.commit().await?;
    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await?;

    Ok(HttpResponse::Ok().finish())
}

/// Insert a subscriber into the database and return its id
// TODO: What happens if a user tries to subscribe twice? Make sure that they receive two confirmation emails
#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"INSERT INTO subscriptions (id, email, name, subscribed_at, status) VALUES ($1, $2, $3, $4, 'pending_confirmation')"#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
    })?;
    Ok(subscriber_id)
}

/// Generate a pseudo-random subscription token
fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(32)
        .collect()
}

/// Store token error type
pub struct StoreTokenError(sqlx::Error);

impl fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl Display for StoreTokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "A database error has occurred while trying to store a subscription token"
        )
    }
}

impl error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.0)
    }
}

impl ResponseError for StoreTokenError {}

/// Store subscription token in the database
#[tracing::instrument(
    name = "Storing subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    subscriber_id: Uuid,
    subscription_token: &str,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    );
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        StoreTokenError(e)
    })?;
    Ok(())
}

/// Send confirmation email to a new subscriber
#[tracing::instrument(
    name = "Sending confirmation email to new subscriber",
    skip(email_client, new_subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link =
        format!("{base_url}/subscriptions/confirm?subscription_token={subscription_token}");
    let html_body = &format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription."
    );
    let text_body = &format!(
        "Welcome to our newsletter!\nVisit {confirmation_link} to confirm your subscription."
    );

    email_client
        .send_email(new_subscriber.email, "Welcome!", html_body, text_body)
        .await
}

/// Provide a representation for any type that implements `Error`
fn error_chain_fmt(e: &impl error::Error, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    writeln!(f, "{e}\n")?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{cause}")?;
        current = cause.source();
    }
    Ok(())
}
