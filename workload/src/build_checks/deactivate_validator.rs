use crate::{
    check::{Check, ValidatorStatus},
    entities::Alias,
};

pub async fn deactivate_validator_build_checks(
    alias: &Alias,
) -> Vec<Check> {
    vec![Check::ValidatorStatus(alias.clone(), ValidatorStatus::Inactive)]
}
