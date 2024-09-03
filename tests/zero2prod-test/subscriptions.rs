use crate::helpers::spawn_app;
use linkify::{LinkFinder, LinkKind};
use sqlx::PgPool;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[sqlx::test]
async fn subscribe_returns_a_200_for_valid_form_data(db_pool: PgPool) {
    let app = spawn_app(db_pool.clone()).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(200, response.status());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&db_pool)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[sqlx::test]
async fn subscribe_returns_a_400_when_data_is_missing(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    #[allow(unused_variables)]
    for (body, description) in test_cases {
        let response = app.post_subscriptions(body.into()).await;

        assert_eq!(
            400,
            response.status(),
            "The API did not fail with 400 Bad Request when the payload was {description}"
        )
    }
}

#[sqlx::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    #[allow(unused_variables)]
    for (body, description) in test_cases {
        let response = app.post_subscriptions(body.into()).await;

        assert_eq!(
            400,
            response.status(),
            "The API did not return a 400 Bad Request when the payload was {description}",
        );
    }
}

#[sqlx::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(200, response.status());
}

#[sqlx::test]
async fn subscribe_sends_a_confirmation_email_with_a_link(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(200, response.status());

    // Get the first request and parse the body as JSON
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    // Extract the link
    let get_link = |s| {
        let links: Vec<_> = LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };

    // Ensure HTML and Text links are identical
    let html_link = get_link(body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(body["TextBody"].as_str().unwrap());
    assert_eq!(html_link, text_link);
}
