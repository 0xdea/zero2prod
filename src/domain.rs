pub struct NewSubscriber {
    pub email: String,
    pub name: SubscriberName,
}

pub struct SubscriberName(String);

impl SubscriberName {
    /// Parse subscriber name
    pub fn parse(name: String) -> Self {
        let is_empty = name.trim().is_empty();
        let is_too_long = name.chars().count() > 256;

        // TODO: it would be better to use a whitelist approach instead
        let blacklisted_chars = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_blacklisted_chars = name.chars().any(|c| blacklisted_chars.contains(&c));

        if is_empty || is_too_long || contains_blacklisted_chars {
            panic!("{name} is not a valid subscriber name.");
        } else {
            Self(name)
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
