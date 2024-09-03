use crate::helpers::spawn_app;
use linkify::{LinkFinder, LinkKind};
use reqwest::{get, Url};
use sqlx::PgPool;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[sqlx::test]
async fn confirmations_without_token_are_rejected_with_a_400(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;

    let response = get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status(), 400);
}

#[sqlx::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called(db_pool: PgPool) {
    let app = spawn_app(db_pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

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
    let raw_link = &get_link(body["HtmlBody"].as_str().unwrap());
    let mut link = Url::parse(raw_link).unwrap();
    // TODO
    link.set_port(Some(app.port)).unwrap();
    assert_eq!(link.host_str().unwrap(), "127.0.0.1");

    let response = get(link).await.unwrap();
    assert_eq!(response.status(), 200)
}
