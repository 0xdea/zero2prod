use std::fmt::{Display, Formatter};
use uuid::Uuid;

/// Idempotency key
#[derive(Debug, serde::Serialize)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Generate idempotency key
    /// TODO: update when we are ready with the backend implementation
    pub fn generate() -> Self {
        Self(Uuid::new_v4().into())
    }
}

impl Display for IdempotencyKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for IdempotencyKey {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            anyhow::bail!("The idempotency key cannot be empty");
        }

        let max_len = 50;
        if value.len() > max_len {
            anyhow::bail!("The idempotency key must be shorter than {max_len} characters");
        }

        Ok(Self(value))
    }
}

impl From<IdempotencyKey> for String {
    fn from(value: IdempotencyKey) -> Self {
        value.0
    }
}

impl AsRef<str> for IdempotencyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
