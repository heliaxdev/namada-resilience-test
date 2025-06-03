use crate::error::{CheckError, StepError, TaskError};
use crate::step::{StepContext, StepType};

const CONNECTION_ERROR_MESSAGE: &str = "connection closed before message completed";

pub enum Code {
    Success(StepType),
    // Fatal failures
    Fatal(StepType, CheckError),
    // No execution
    Skip(StepType),
    NoTask(StepType),
    // Other failures
    StepFailure(StepType, StepError),
    TaskFailure(StepType, TaskError),
    CheckFailure(StepType, CheckError),
}

pub enum CodeType {
    Success,
    Fatal,
    Skip,
    AcceptableFailure,
    UnexpectedFailure,
}

impl Code {
    pub fn code(&self) -> i32 {
        match self {
            Code::Fatal(_, _) => 1,
            // system state is as expected
            _ => 0,
        }
    }

    pub fn step_type(&self) -> &StepType {
        match self {
            Code::Success(st) => st,
            Code::Fatal(st, _) => st,
            Code::Skip(st) => st,
            Code::NoTask(st) => st,
            Code::StepFailure(st, _) => st,
            Code::TaskFailure(st, _) => st,
            Code::CheckFailure(st, _) => st,
        }
    }

    pub fn output_logs(&self) {
        match self {
            Code::Success(step_type) => tracing::info!("Done {step_type} successfully!"),
            Code::Fatal(step_type, reason) => {
                tracing::error!("State check error for {step_type} -> {reason}")
            }
            Code::StepFailure(step_type, reason) => {
                tracing::error!("Step failure for {step_type} -> {reason}")
            }
            Code::TaskFailure(step_type, reason) => {
                tracing::error!("Task failure for {step_type} -> {reason}")
            }
            Code::CheckFailure(step_type, reason) => {
                tracing::error!("Check failure for {step_type} -> {reason}")
            }
            Code::Skip(step_type) => {
                tracing::warn!("Invalid step for {step_type}, skipping...")
            }
            Code::NoTask(step_type) => tracing::info!("No task for {step_type}, skipping..."),
        }
    }

    pub fn code_type(&self) -> CodeType {
        match self {
            Code::Success(_) => CodeType::Success,
            Code::Fatal(_, _) => CodeType::Fatal,
            Code::Skip(_) | Code::NoTask(_) => CodeType::Skip,
            Code::TaskFailure(_, err) if is_acceptable_failure(err) => CodeType::AcceptableFailure,
            _ => CodeType::UnexpectedFailure,
        }
    }

    pub fn details(&self) -> String {
        let (step_type, outcome, error) = match self {
            Code::Success(step_type) => (step_type, "Success", Default::default()),
            Code::Fatal(step_type, e) => (step_type, "Fatal failure", e.to_string()),
            Code::Skip(step_type) => (step_type, "Skipped step", Default::default()),
            Code::NoTask(step_type) => (step_type, "No task", Default::default()),
            Code::StepFailure(step_type, e) => (step_type, "Step failure", e.to_string()),
            Code::TaskFailure(step_type, e) => (step_type, "Task failure", e.to_string()),
            Code::CheckFailure(step_type, e) => (step_type, "Check failure", e.to_string()),
        };
        let details = serde_json::json!({
            "step_type": step_type.name(),
            "outcome": outcome,
            "error": error
        });
        serde_json::to_string_pretty(&details).expect("Details should be convertible")
    }
}

fn is_acceptable_failure(err: &TaskError) -> bool {
    match err {
        TaskError::IbcTransfer(_) | TaskError::InvalidShielded { .. } => true,
        TaskError::BuildTx(e) if e.to_string().contains(CONNECTION_ERROR_MESSAGE) => true,
        TaskError::Broadcast(e) if e.to_string().contains(CONNECTION_ERROR_MESSAGE) => true,
        _ => false,
    }
}
