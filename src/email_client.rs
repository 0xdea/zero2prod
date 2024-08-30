use crate::domain::EmailAddress;
use reqwest::Client;
use url::Url;

/// Email client data
pub struct EmailClient {
    http_client: Client,
    base_url: Url,
    sender: EmailAddress,
}

impl EmailClient {
    pub fn new(base_url: Url, sender: EmailAddress) -> Self {
        Self {
            http_client: Client::new(),
            base_url,
            sender,
        }
    }

    /// Send an email using Postmark's REST API
    /// <https://postmarkapp.com/developer/user-guide/send-email-with-api>
    pub async fn send_email(
        &self,
        recipient: EmailAddress,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), String> {
        let url = self.base_url.join("/email").expect("Cannot parse URL");
        let builder = self.http_client.post(url.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::Fake;
    use wiremock::matchers::any;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let sender_email = EmailAddress::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(mock_server.uri().parse().unwrap(), sender_email);

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
