use std::fmt;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lower")]
pub enum RuntimeStatus {
    Idle,
    Running,
    Hibernated,
    Failed,
}

impl fmt::Display for RuntimeStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Hibernated => "hibernated",
            Self::Failed => "failed",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct EnvironmentSpec {
    pub kind: String,
    pub executable: String,
    pub digest: Option<String>,
}

impl Default for EnvironmentSpec {
    fn default() -> Self {
        Self {
            kind: "python".to_owned(),
            executable: "python3".to_owned(),
            digest: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SnapshotPolicy {
    pub interval_executions: u32,
    pub mode: String,
}

impl Default for SnapshotPolicy {
    fn default() -> Self {
        Self {
            interval_executions: 25,
            mode: "logical".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeCreate {
    pub name: String,
    pub language: String,
    pub environment: EnvironmentSpec,
    pub snapshot_policy: SnapshotPolicy,
}

impl RuntimeCreate {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            language: "python".to_owned(),
            environment: EnvironmentSpec::default(),
            snapshot_policy: SnapshotPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct RuntimeUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<EnvironmentSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_policy: Option<SnapshotPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<RuntimeStatus>,
}

impl RuntimeUpdate {
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.language.is_none()
            && self.environment.is_none()
            && self.snapshot_policy.is_none()
            && self.status.is_none()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Runtime {
    pub id: String,
    pub name: String,
    pub language: String,
    pub environment: EnvironmentSpec,
    pub snapshot_policy: SnapshotPolicy,
    pub status: RuntimeStatus,
    pub worker_generation: u64,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct RuntimeList {
    pub items: Vec<Runtime>,
    pub total: u64,
    pub limit: u32,
    pub offset: u64,
}
