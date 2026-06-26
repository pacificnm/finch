use crate::types::NotificationId;

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: NotificationId,
    pub level: NotificationLevel,
    pub title: String,
    pub message: Option<String>,
    pub source: NotificationSource,
    pub read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub enum NotificationSource {
    Core,
    Task(crate::types::TaskId),
    Plugin(crate::types::PluginId),
}

#[derive(Debug, Clone, Default)]
pub struct NotificationStore {
    notifications: Vec<Notification>,
    next_id: u64,
}

impl NotificationStore {
    pub fn push(&mut self, mut notification: Notification) {
        self.next_id += 1;
        notification.id = NotificationId(self.next_id);
        self.notifications.push(notification);
    }

    pub fn all(&self) -> &[Notification] {
        &self.notifications
    }

    pub fn unread_count(&self) -> usize {
        self.notifications.iter().filter(|n| !n.read).count()
    }
}
