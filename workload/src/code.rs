use crate::error::{CheckError, StepError, TaskError};
use crate::state::StateError;
use crate::step::{StepContext, StepType};

pub enum Code {
    Success(StepType),
    Fatal(StepType, CheckError),
    StepFailure(StepType, StepError),
    TaskFailure(StepType, TaskError),
    CheckFailure(StepType, CheckError),
    InvalidStep(StepType),
    NoTask(StepType),
    StateFatal(StateError),
    InitFatal(StepError),
}

impl Code {
    pub fn code(&self) -> i32 {
        match self {
            Code::Success(_) => 0,
            Code::Fatal(_, _) => 1,
            Code::StepFailure(_, _) => 2,
            Code::TaskFailure(_, _) => 4,
            Code::CheckFailure(_, _) => 5,
            Code::NoTask(_) => 6,
            Code::StateFatal(_) => 8,
            Code::InitFatal(_) => 9,
            Code::InvalidStep(_) => 10,
        }
    }

    pub fn step_type(&self) -> Option<&StepType> {
        match self {
            Code::Success(st) => Some(st),
            Code::Fatal(st, _) => Some(st),
            Code::StepFailure(st, _) => Some(st),
            Code::TaskFailure(st, _) => Some(st),
            Code::CheckFailure(st, _) => Some(st),
            Code::InvalidStep(st) => Some(st),
            Code::NoTask(st) => Some(st),
            Code::StateFatal(_) => None,
            Code::InitFatal(_) => None,
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
            Code::InvalidStep(step_type) => {
                tracing::warn!("Invalid step for {step_type}, skipping...")
            }
            Code::NoTask(step_type) => tracing::info!("No task for {step_type}, skipping..."),
            Code::StateFatal(reason) => {
                tracing::error!("State error -> {reason}")
            }
            Code::InitFatal(reason) => {
                tracing::error!("Init error -> {reason}")
            }
        }
    }

    pub fn is_fatal(&self) -> bool {
        matches!(self, Code::Fatal(_, _) | Code::StateFatal(_))
    }

    pub fn is_failed(&self) -> bool {
        !(self.is_fatal() || self.is_successful())
    }

    pub fn is_skipped(&self) -> bool {
        matches!(self, Code::InvalidStep(_))
    }

    pub fn is_successful(&self) -> bool {
        matches!(self, Code::Success(_))
    }

    pub fn assert(&self) {
        if let Some(step_type) = self.step_type() {
            step_type.assert(self);
        }
    }
}
