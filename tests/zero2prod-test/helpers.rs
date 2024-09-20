use std::{env, io, sync};

use fake::faker::internet::en::{Password, Username};
use fake::Fake;
use linkify::{LinkFinder, LinkKind};
use reqwest::Url;
use secrecy::{ExposeSecret, SecretBox};
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use zero2prod::configuration::get_config;
use zero2prod::routes::Credentials;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

/// Ensure the tracing stack is initialized only once
static TRACING: sync::LazyLock<()> = sync::LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if env::var("TEST_LOG").is_ok() {
        init_subscriber(get_subscriber(
            subscriber_name,
            default_filter_level,
            io::stdout,
        ));
    } else {
        init_subscriber(get_subscriber(
            subscriber_name,
            default_filter_level,
            io::sink,
        ));
    };
});

/// Test application data
pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub email_server: MockServer,
    pub creds: Credentials,
}

/// Confirmation links
pub struct ConfirmationLinks {
    pub html: Url,
    pub text: Url,
}

impl TestApp {
    /// Spin up a test application and return its data
    pub async fn spawn(db_pool: PgPool) -> Self {
        // Initialize logging
        sync::LazyLock::force(&TRACING);

        // Launch a mock server to stand in for Postmark's API
        let email_server = MockServer::start().await;

        // Get settings and modify them for testing
        let config = {
            let mut c = get_config().expect("Failed to read configuration");
            // Listen on a random TCP port
            c.application.app_port = 0;
            // Use the mock server as email API
            c.email_client.base_url = email_server.uri();
            c
        };

        // Add test user
        let creds = add_test_user(&db_pool).await;

        // Build the application and get its address
        let app = Application::build_with_db_pool(config, db_pool)
            .await
            .expect("Failed to build application");
        let port = app.port();
        let address = format!("http://127.0.0.1:{port}");

        // Run the application and return its data
        #[allow(clippy::let_underscore_future)]
        let _ = tokio::spawn(app.run_until_stopped());
        Self {
            address,
            port,
            email_server,
            creds,
        }
    }

    /// Perform a POST request to the subscriptions endpoint
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to send request")
    }

    /// Extract confirmation links embedded in the request to the email API
    pub fn confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        // Parse the request body as JSON
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        // Extract the link
        let get_link = |s| {
            let links: Vec<_> = LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut link = Url::parse(&raw_link).unwrap();
            assert_eq!(link.host_str().unwrap(), "127.0.0.1");
            link.set_port(Some(self.port)).unwrap();
            link
        };

        // Return the extracted links
        let html_link = get_link(body["HtmlBody"].as_str().unwrap());
        let text_link = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks {
            html: html_link,
            text: text_link,
        }
    }

    /// Create an unconfirmed subscriber using the public API
    pub async fn create_unconfirmed_subscriber(&self) -> ConfirmationLinks {
        let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

        // Build a scoped mock Postmark server
        let _mock_guard = Mock::given(path("/email"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .named("Create unconfirmed subscriber")
            .expect(1)
            .mount_as_scoped(&self.email_server)
            .await;

        // Subscribe to the newsletter using the API
        self.post_subscriptions(body.into())
            .await
            .error_for_status()
            .unwrap();

        // Inspect the requests received by the mock server to retrieve the confirmation link and return it
        let email_request = &self
            .email_server
            .received_requests()
            .await
            .unwrap()
            .pop()
            .unwrap();
        self.confirmation_links(email_request)
    }

    /// Create a confirmed subscriber using the public API
    pub async fn create_confirmed_subscriber(&self) {
        // Reuse the helper that creates an unconfirmed subscriber
        let confirmation_link = self.create_unconfirmed_subscriber().await;

        // Confirm subscription to the newsletter using the API
        reqwest::get(confirmation_link.html)
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }

    /// POST to the newsletters endpoint
    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        let (username, password) = self.test_user();
        reqwest::Client::new()
            .post(format!("{}/newsletters", &self.address))
            .basic_auth(username, Some(password))
            .json(&body)
            .send()
            .await
            .expect("Failed to send request")
    }

    /// Get username and password for the test app
    pub fn test_user(&self) -> (String, String) {
        (
            self.creds.username.to_string(),
            self.creds.password.expose_secret().to_string(),
        )
    }
}

/// Add test user to the database and return its credentials
async fn add_test_user(db_pool: &PgPool) -> Credentials {
    let username = Username().fake::<String>();
    let password = Password(32..33).fake::<String>();

    sqlx::query!(
        "INSERT INTO users (user_id, username, password)\
        VALUES ($1, $2, $3)",
        Uuid::new_v4(),
        username,
        password,
    )
    .execute(db_pool)
    .await
    .expect("Failed to add test user");

    Credentials {
        username,
        password: SecretBox::from(Box::new(password)),
    }
}
