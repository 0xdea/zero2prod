use crate::domain::{Email, SubscriberName};

/// New subscriber data
pub struct NewSubscriber {
    pub email: Email,
    pub name: SubscriberName,
}
