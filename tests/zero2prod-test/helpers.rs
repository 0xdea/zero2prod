use linkify::{LinkFinder, LinkKind};
use reqwest::{Client, Url};
use sqlx::PgPool;
use wiremock::MockServer;
use zero2prod::configuration::get_config;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

/// Ensure the tracing stack is initialized only once
static TRACING: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        init_subscriber(get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::stdout,
        ));
    } else {
        init_subscriber(get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::sink,
        ));
    };
});

/// Test application data
pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub email_server: MockServer,
}

/// Confirmation links
pub struct ConfirmationLinks {
    pub html_link: Url,
    pub text_link: Url,
}

impl TestApp {
    /// Perform a POST request to the subscriptions endpoint
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        Client::new()
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    /// Extract confirmation links embedded in the request to the email API
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
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
            html_link,
            text_link,
        }
    }
}

/// Spin up a test application and return its data
pub async fn spawn_app(db_pool: PgPool) -> TestApp {
    // Initialize logging
    std::sync::LazyLock::force(&TRACING);

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

    // Build the application and get its address
    let app = Application::build_with_db_pool(config, db_pool)
        .await
        .expect("Failed to build application");
    let port = app.port();
    let address = format!("http://127.0.0.1:{}", port);

    // Run the application and return its data
    #[allow(clippy::let_underscore_future)]
    let _ = tokio::spawn(app.run_until_stopped());
    TestApp {
        address,
        port,
        email_server,
    }
}
