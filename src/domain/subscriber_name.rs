/// Forbidden chars for subscriber name
// TODO: it would be better to use a whitelist approach instead
const NAME_BLACKLIST: [char; 9] = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];

/// Subscriber name
#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    /// Parse subscriber name
    pub fn parse(name: String) -> Result<Self, String> {
        let is_empty = name.trim().is_empty();
        let is_too_long = name.chars().count() > 256;
        let contains_blacklisted_chars = name.chars().any(|c| NAME_BLACKLIST.contains(&c));

        if is_empty || is_too_long || contains_blacklisted_chars {
            Err(format!("{name} is not a valid subscriber name"))
        } else {
            Ok(Self(name))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use claim::{assert_err, assert_ok};

    use super::*;

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "a".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = String::new();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &NAME_BLACKLIST {
            let name = name.to_string();
            assert_err!(SubscriberName::parse(name));
        }
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
