use crate::domain::{EmailAddress, SubscriberName};

/// New subscriber data
pub struct NewSubscriber {
    pub email: EmailAddress,
    pub name: SubscriberName,
}
