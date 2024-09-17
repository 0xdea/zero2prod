use std::time;

use reqwest::{Client, Url};
use secrecy::{ExposeSecret, Secret};

use crate::domain::EmailAddress;

/// Send email request data
#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

/// Email client data
pub struct EmailClient {
    http_client: Client,
    base_url: Url,
    sender: EmailAddress,
    authorization_token: Secret<String>,
}

// TODO: Use a proper templating solution for our emails (e.g. tera)
impl EmailClient {
    pub fn new(
        base_url: Url,
        sender: EmailAddress,
        authorization_token: Secret<String>,
        timeout: time::Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            http_client,
            base_url,
            sender,
            authorization_token,
        }
    }

    /// Send an email using Postmark's REST API
    /// <https://postmarkapp.com/developer/user-guide/send-email-with-api>
    pub async fn send_email(
        &self,
        to: &EmailAddress,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<(), reqwest::Error> {
        let url = self.base_url.join("/email").expect("Cannot parse URL");
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: to.as_ref(),
            subject,
            html_body,
            text_body,
        };

        self.http_client
            .post(url.to_string())
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_ok};
    use fake::faker::internet::en::Password;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::Fake;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    use super::*;

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result = serde_json::from_slice::<serde_json::Value>(&request.body);
            result.map_or(false, |body| {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            })
        }
    }

    /// Generate random email address
    fn email() -> EmailAddress {
        EmailAddress::parse(SafeEmail().fake()).unwrap()
    }

    /// Generate random email subject
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    /// Generate random email content
    fn content() -> String {
        Paragraph(1..10).fake()
    }

    /// Get a test instance of email client
    fn email_client(base_url: Url) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            Secret::new(Password(32..33).fake()),
            time::Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri().parse().unwrap());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri().parse().unwrap());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri().parse().unwrap());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri().parse().unwrap());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200).set_delay(time::Duration::from_secs(180)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;
        assert_err!(outcome);
    }
}
