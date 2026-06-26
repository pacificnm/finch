macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub u64);
    };
}

id_type!(WorkspaceId);
id_type!(PanelId);
id_type!(TaskId);
id_type!(NotificationId);
id_type!(CommandId);
id_type!(PluginId);
id_type!(TabId);
