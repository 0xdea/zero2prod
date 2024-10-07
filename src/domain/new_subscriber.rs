use crate::domain::{EmailAddress, SubscriberName};

/// New subscriber
pub struct NewSubscriber {
    pub email: EmailAddress,
    pub name: SubscriberName,
}
