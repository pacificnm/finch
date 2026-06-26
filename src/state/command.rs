use crate::types::CommandId;

#[derive(Debug, Clone)]
pub struct CommandEntry {
    pub id: CommandId,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CommandStore {
    commands: Vec<CommandEntry>,
}

impl CommandStore {
    pub fn register(&mut self, entry: CommandEntry) {
        self.commands.push(entry);
    }

    pub fn search(&self, query: &str) -> Vec<&CommandEntry> {
        self.commands
            .iter()
            .filter(|c| c.name.contains(query))
            .collect()
    }
}
