use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Analysis,
    Translation,
    Import,
    Indexing,
}

impl JobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Analysis => "analysis",
            Self::Translation => "translation",
            Self::Import => "import",
            Self::Indexing => "indexing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Result<Self, JobTransitionError> {
        match value.trim() {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(JobTransitionError::UnknownStatus(other.to_string())),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum JobTransitionError {
    #[error("unknown job status: {0}")]
    UnknownStatus(String),

    #[error("invalid job transition from {from} to {to}")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },
}

pub fn validate_transition(from: JobStatus, to: JobStatus) -> Result<(), JobTransitionError> {
    if from == to {
        return Ok(());
    }

    let allowed = matches!(
        (from, to),
        (JobStatus::Pending, JobStatus::Running)
            | (JobStatus::Pending, JobStatus::Cancelled)
            | (JobStatus::Running, JobStatus::Completed)
            | (JobStatus::Running, JobStatus::Failed)
            | (JobStatus::Running, JobStatus::Cancelled)
            | (JobStatus::Failed, JobStatus::Pending)
    );

    if allowed {
        Ok(())
    } else {
        Err(JobTransitionError::InvalidTransition {
            from: from.as_str(),
            to: to.as_str(),
        })
    }
}

pub fn cancel_event_type(kind: JobKind) -> &'static str {
    match kind {
        JobKind::Analysis => "analysis_job_cancelled",
        JobKind::Translation => "translation_job_cancelled",
        JobKind::Import => "import_job_cancelled",
        JobKind::Indexing => "indexing_job_cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_transition, JobStatus, JobTransitionError};

    #[test]
    fn allows_expected_state_transitions() {
        assert!(validate_transition(JobStatus::Pending, JobStatus::Running).is_ok());
        assert!(validate_transition(JobStatus::Pending, JobStatus::Cancelled).is_ok());
        assert!(validate_transition(JobStatus::Running, JobStatus::Completed).is_ok());
        assert!(validate_transition(JobStatus::Running, JobStatus::Failed).is_ok());
        assert!(validate_transition(JobStatus::Running, JobStatus::Cancelled).is_ok());
        assert!(validate_transition(JobStatus::Failed, JobStatus::Pending).is_ok());
    }

    #[test]
    fn rejects_terminal_state_mutation() {
        assert_eq!(
            validate_transition(JobStatus::Completed, JobStatus::Cancelled),
            Err(JobTransitionError::InvalidTransition {
                from: "completed",
                to: "cancelled"
            })
        );
        assert_eq!(
            validate_transition(JobStatus::Cancelled, JobStatus::Running),
            Err(JobTransitionError::InvalidTransition {
                from: "cancelled",
                to: "running"
            })
        );
    }
}
