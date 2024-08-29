use crate::domain::Email;
use reqwest::Client;

/// Email client data
pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: Email,
}

impl EmailClient {
    pub fn new(base_url: String, sender: Email) -> Self {
        Self {
            http_client: Client::new(),
            base_url,
            sender,
        }
    }

    /// Send an email
    pub async fn send_email(
        &self,
        recipient: Email,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), String> {
        todo!();
    }
}
