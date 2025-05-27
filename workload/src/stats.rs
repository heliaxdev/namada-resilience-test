use std::collections::HashMap;
use std::thread::ThreadId;

use crate::code::{Code, CodeType};
use crate::error::TaskError;
use crate::step::StepType;
use crate::types::StepId;

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub success: HashMap<StepType, u64>,
    pub fatal: HashMap<StepType, u64>,
    pub skip: HashMap<StepType, u64>,
    pub acceptable_failures: HashMap<StepType, u64>,
    pub unexpected_failures: HashMap<StepType, u64>,
    pub fatal_failure_logs: HashMap<StepId, String>,
    pub acceptable_failure_logs: HashMap<StepId, String>,
    pub unexpected_failure_logs: HashMap<StepId, String>,
}

impl Stats {
    pub fn update(&mut self, id: StepId, code: &Code) {
        match code.code_type() {
            CodeType::Success => {
                self.success
                    .entry(code.step_type().clone())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
            }
            CodeType::Skip => {
                self.skip
                    .entry(code.step_type().clone())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
            }
            CodeType::Fatal => {
                self.fatal
                    .entry(code.step_type().clone())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
                self.fatal_failure_logs.insert(id, code.details());
            }
            CodeType::Failed => {
                if matches!(
                    code,
                    Code::TaskFailure(_, TaskError::IbcTransfer(_))
                        | Code::TaskFailure(_, TaskError::InvalidShielded { .. })
                ) {
                    self.acceptable_failures
                        .entry(code.step_type().clone())
                        .and_modify(|count| *count += 1)
                        .or_insert(1);
                    self.acceptable_failure_logs.insert(id, code.details());
                } else {
                    self.unexpected_failures
                        .entry(code.step_type().clone())
                        .and_modify(|count| *count += 1)
                        .or_insert(1);
                    self.unexpected_failure_logs.insert(id, code.details());
                }
            }
        }
    }

    pub fn report(&self, thread_id: ThreadId) {
        let result = if !self.fatal.is_empty() {
            "Fatal failures happened"
        } else if !self.unexpected_failures.is_empty() {
            "Non-fatal failures happened"
        } else if self.success.is_empty() {
            "No successful transaction"
        } else {
            "Done successfully"
        };
        tracing::info!("==== {thread_id:?} Result: {result} ====");

        tracing::info!("{self:#?}")
    }
}
