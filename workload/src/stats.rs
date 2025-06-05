use std::collections::HashMap;

use crate::code::{Code, CodeType};
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
    pub pre_balance_check_failures: HashMap<StepId, HashMap<String, serde_json::Value>>,
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
            CodeType::AcceptableFailure => {
                self.acceptable_failures
                    .entry(code.step_type().clone())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
                self.acceptable_failure_logs.insert(id, code.details());
            }
            CodeType::UnexpectedFailure => {
                self.unexpected_failures
                    .entry(code.step_type().clone())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
                self.unexpected_failure_logs.insert(id, code.details());
            }
        }
    }
}

impl std::fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "==== {:?} Stats ====", std::thread::current().id())?;
        writeln!(f, "-- Success --")?;
        for (step_type, count) in self.success.iter() {
            writeln!(f, "  - {step_type}: {count}")?;
        }
        writeln!(f, "-- Fatal --")?;
        for (step_type, count) in self.fatal.iter() {
            writeln!(f, "  - {step_type}: {count}")?;
        }
        writeln!(f, "-- Skip --")?;
        for (step_type, count) in self.skip.iter() {
            writeln!(f, "  - {step_type}: {count}")?;
        }
        writeln!(f, "-- Acceptable Failure --")?;
        for (step_type, count) in self.acceptable_failures.iter() {
            writeln!(f, "  - {step_type}: {count}")?;
        }
        writeln!(f, "-- Unexpected Failure --")?;
        for (step_type, count) in self.unexpected_failures.iter() {
            writeln!(f, "  - {step_type}: {count}")?;
        }

        writeln!(f, "----------------")?;

        writeln!(f, "-- Fatal Failure Logs --")?;
        for (id, details) in self.fatal_failure_logs.iter() {
            writeln!(f, "  - {id}: {details}")?;
        }
        writeln!(f, "-- Acceptable Failure Logs --")?;
        for (id, details) in self.acceptable_failure_logs.iter() {
            writeln!(f, "  - {id}: {details}")?;
        }
        writeln!(f, "-- Unexpected Failure Logs --")?;
        for (id, details) in self.unexpected_failure_logs.iter() {
            writeln!(f, "  - {id}: {details}")?;
        }
        writeln!(f, "-- Pre-balance Check Failure Logs --")?;
        for (id, details) in self.pre_balance_check_failures.iter() {
            writeln!(f, "  - {id}:")?;
            for (check_type, info) in details {
                writeln!(f, "    - {check_type}:")?;
                let pretty = serde_json::to_string_pretty(info).expect("infallible");
                for line in pretty.lines() {
                    writeln!(f, "      {line}")?;
                }
            }
        }

        Ok(())
    }
}

pub fn summary_stats(stats: Vec<Stats>, output: bool) -> bool {
    let mut success = HashMap::new();
    let mut fatal = HashMap::new();
    let mut skip = HashMap::new();
    let mut acceptable_failures = HashMap::new();
    let mut unexpected_failures = HashMap::new();
    let all_prebalance_correct = stats
        .iter()
        .all(|s| s.pre_balance_check_failures.is_empty());
    for s in stats {
        for (st, v) in &s.success {
            *success.entry(st.to_string()).or_insert(0) += *v;
        }
        for (st, v) in &s.fatal {
            *fatal.entry(st.to_string()).or_insert(0) += *v;
        }
        for (st, v) in &s.skip {
            *skip.entry(st.to_string()).or_insert(0) += *v;
        }
        for (st, v) in &s.acceptable_failures {
            *acceptable_failures.entry(st.to_string()).or_insert(0) += *v;
        }
        for (st, v) in &s.unexpected_failures {
            *unexpected_failures.entry(st.to_string()).or_insert(0) += *v;
        }
    }

    let (summary, is_successful) = if !fatal.is_empty() {
        ("Fatal failures happened", false)
    } else if !unexpected_failures.is_empty() {
        ("Non-fatal failures happened", false)
    } else if !all_prebalance_correct {
        ("Pre-balance check failure happened", false)
    } else if success.is_empty() {
        ("No successful transaction", false)
    } else {
        ("Done successfully", true)
    };

    if output {
        println!("==== Summary: {summary} ====");
        println!("-- Success --");
        for (step_type, count) in success.iter() {
            println!("  - {step_type}: {count}");
        }
        println!("-- Fatal --");
        for (step_type, count) in fatal.iter() {
            println!("  - {step_type}: {count}");
        }
        println!("-- Skip --");
        for (step_type, count) in skip.iter() {
            println!("  - {step_type}: {count}");
        }
        println!("-- Acceptable Failure --");
        for (step_type, count) in acceptable_failures.iter() {
            println!("  - {step_type}: {count}");
        }
        println!("-- Unexpected Failure --");
        for (step_type, count) in unexpected_failures.iter() {
            println!("  - {step_type}: {count}");
        }
    }

    is_successful
}
