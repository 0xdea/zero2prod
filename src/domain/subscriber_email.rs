use std::fmt;

use validator::ValidateEmail;

/// Email address
#[derive(Clone, Debug)]
pub struct EmailAddress(String);

impl EmailAddress {
    /// Parse email address
    pub fn parse(email: String) -> Result<Self, String> {
        if ValidateEmail::validate_email(&email) {
            Ok(Self(email))
        } else {
            Err(format!("{email} is not a valid subscriber email"))
        }
    }
}

impl AsRef<str> for EmailAddress {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use claims::assert_err;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;
    use quickcheck::{Arbitrary, Gen};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use super::*;

    #[test]
    fn empty_string_is_rejected() {
        let email = String::new();
        assert_err!(EmailAddress::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assert_err!(EmailAddress::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(EmailAddress::parse(email));
    }

    #[derive(Clone, Debug)]
    struct ValidEmail(String);

    impl Arbitrary for ValidEmail {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(email: ValidEmail) -> bool {
        // dbg!(&email.0);
        EmailAddress::parse(email.0).is_ok()
    }
}
