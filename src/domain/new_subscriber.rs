use crate::domain::{SubscriberEmail, SubscriberName};

/// New subscriber data
pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
