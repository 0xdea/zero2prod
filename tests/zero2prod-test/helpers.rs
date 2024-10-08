use std::{env, io, sync};

use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use fake::faker::internet::en::{Password, Username};
use fake::Fake;
use fdlimit::raise_fd_limit;
use linkify::{LinkFinder, LinkKind};
use reqwest::Url;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use zero2prod::configuration::Settings;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

use crate::FAKE_PASSWORD_LEN;

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

/// Confirmation links embedded in the request to the email API
pub struct ConfirmationLinks {
    pub html: Url,
    pub text: Url,
}

/// Test application data
pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub email_server: MockServer,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
}

impl TestApp {
    /// Initialize test database pool
    pub async fn init_test_db_pool(conn_opts: PgConnectOptions) -> PgPool {
        PgPoolOptions::new().connect_lazy_with(conn_opts)
    }

    /// Spin up a test application and return its data
    pub async fn spawn(db_pool: &PgPool) -> Self {
        // Initialize logging
        sync::LazyLock::force(&TRACING);

        // Raise file descriptors limit to avoid "Too many open files" error
        raise_fd_limit().expect("Failed to raise fd limit");

        // Launch a mock server to stand in for Postmark's API
        let email_server = MockServer::start().await;

        // Get settings and modify them for testing
        let config = {
            let mut c = Settings::get_config().expect("Failed to read configuration");
            // Listen on a random TCP port
            c.application.app_port = 0;
            // Use the mock server as email API
            c.email_client.base_url = email_server.uri();
            c
        };

        // Add test user
        let test_user = TestUser::generate();
        test_user.store(db_pool).await;

        // Build the application and get its address
        let app = Application::build_with_db_pool(config, db_pool)
            .await
            .expect("Failed to build application");
        let port = app.port();
        let address = format!("http://127.0.0.1:{port}");

        // Build the API client
        let api_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .cookie_store(true)
            .build()
            .unwrap();

        // Run the application and return its data
        #[allow(clippy::let_underscore_future)]
        let _ = tokio::spawn(app.run_until_stopped());
        Self {
            address,
            port,
            email_server,
            test_user,
            api_client,
        }
    }

    /// Perform a POST request to the subscriptions endpoint
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
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
    #[allow(clippy::future_not_send)]
    pub async fn post_newsletters<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(format!("{}/admin/newsletters", &self.address))
            .form(&body)
            .send()
            .await
            .expect("Failed to send request")
    }

    /// GET to the newsletter endpoint
    pub async fn get_newsletters(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/newsletters", &self.address))
            .send()
            .await
            .expect("Failed to send request")
    }

    /// GET to the newsletter endpoint and extract HTML
    pub async fn get_newsletters_html(&self) -> String {
        self.get_newsletters().await.text().await.unwrap()
    }

    /// POST to the login endpoint
    #[allow(clippy::future_not_send)]
    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(format!("{}/login", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to send request")
    }

    /// GET to the login endpoint and extract HTML
    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(format!("{}/login", &self.address))
            .send()
            .await
            .expect("Failed to send request")
            .text()
            .await
            .unwrap()
    }

    /// GET to the admin dashboard endpoint
    pub async fn get_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/dashboard", &self.address))
            .send()
            .await
            .expect("Failed to send request")
    }

    /// GET to the admin dashboard endpoint and extract HTML
    pub async fn get_dashboard_html(&self) -> String {
        self.get_dashboard().await.text().await.unwrap()
    }

    /// GET to the password change endpoint
    pub async fn get_password(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/password", &self.address))
            .send()
            .await
            .expect("Failed to send request")
    }

    /// GET to the password change endpoint and extract HTML
    pub async fn get_password_html(&self) -> String {
        self.get_password().await.text().await.unwrap()
    }

    /// POST to the password change endpoint
    #[allow(clippy::future_not_send)]
    pub async fn post_password<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(format!("{}/admin/password", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to send request")
    }

    /// POST to the logout endpoint
    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(format!("{}/admin/logout", &self.address))
            .send()
            .await
            .expect("Failed to send request")
    }
}

/// Test user data
pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    /// Generate new test `user_id` and authentication credentials
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: fake_username(),
            password: fake_password(FAKE_PASSWORD_LEN),
        }
    }

    /// Store test user data in the database
    async fn store(&self, db_pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        sqlx::query!(
            r#"
            INSERT INTO users (user_id, username, password_hash)
            VALUES ($1, $2, $3)
            "#,
            self.user_id,
            self.username,
            password_hash
        )
        .execute(db_pool)
        .await
        .expect("Failed to store test user in the database");
    }

    /// Login to the test application
    pub async fn login(&self, app: &TestApp) -> reqwest::Response {
        app.post_login(&serde_json::json!({
            "username": &self.username,
            "password": &self.password
        }))
        .await
    }
}

/// Assert: response is a redirect to the specified location
pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}

/// Generate a fake username
pub fn fake_username() -> String {
    Username().fake()
}

/// Generate a fake password
#[allow(clippy::range_plus_one)]
pub fn fake_password(len: usize) -> String {
    Password(len..len + 1).fake()
}
