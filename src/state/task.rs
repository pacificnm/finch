use std::time::SystemTime;

use crate::action::{TaskOutcome, TaskRequest, TaskResult};
use crate::types::TaskId;

#[derive(Debug, Clone)]
pub struct TaskState {
    pub id: TaskId,
    pub label: String,
    pub status: TaskStatus,
    pub owner: crate::action::TaskOwner,
    pub started_at: SystemTime,
    pub last_update_at: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Default)]
pub struct TaskStore {
    tasks: Vec<TaskState>,
    next_id: u64,
}

impl TaskStore {
    pub fn register(&mut self, request: TaskRequest) -> TaskId {
        self.next_id += 1;
        let id = TaskId(self.next_id);
        let now = SystemTime::now();
        self.tasks.push(TaskState {
            id,
            label: request.label,
            status: TaskStatus::Pending,
            owner: request.owner,
            started_at: now,
            last_update_at: now,
        });
        id
    }

    pub fn complete(&mut self, result: TaskResult) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == result.task_id) {
            task.status = match result.outcome {
                TaskOutcome::Success => TaskStatus::Completed,
                TaskOutcome::Failure(_) => TaskStatus::Failed,
                TaskOutcome::Cancelled => TaskStatus::Cancelled,
            };
            task.last_update_at = SystemTime::now();
        }
    }

    pub fn cancel(&mut self, id: TaskId) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.status = TaskStatus::Cancelled;
            task.last_update_at = SystemTime::now();
        }
    }

    pub fn get(&self, id: TaskId) -> Option<&TaskState> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn active_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Pending | TaskStatus::Running))
            .count()
    }
}
