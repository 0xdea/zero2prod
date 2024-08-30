use crate::domain::EmailAddress;
use reqwest::{Client, Url};
use secrecy::{ExposeSecret, Secret};

/// Send email request data
#[derive(serde::Serialize)]
struct SendEmailRequest {
    from: String,
    to: String,
    subject: String,
    html_body: String,
    text_body: String,
}

/// Email client data
pub struct EmailClient {
    http_client: Client,
    base_url: Url,
    sender: EmailAddress,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(base_url: Url, sender: EmailAddress, authorization_token: Secret<String>) -> Self {
        Self {
            http_client: Client::new(),
            base_url,
            sender,
            authorization_token,
        }
    }

    /// Send an email using Postmark's REST API
    /// <https://postmarkapp.com/developer/user-guide/send-email-with-api>
    pub async fn send_email(
        &self,
        to: EmailAddress,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<(), reqwest::Error> {
        let url = self.base_url.join("/email").expect("Cannot parse URL");
        let request_body = SendEmailRequest {
            from: self.sender.as_ref().to_owned(),
            to: to.as_ref().to_owned(),
            subject: subject.to_owned(),
            html_body: html_body.to_owned(),
            text_body: text_body.to_owned(),
        };
        self.http_client
            .post(url.to_string())
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake::faker::internet::en::Password;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::Fake;
    use wiremock::matchers::any;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let sender_email = EmailAddress::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            mock_server.uri().parse().unwrap(),
            sender_email,
            Secret::new(Password(8..17).fake()),
        );

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = EmailAddress::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();
        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
    }
}
