use std::collections::BTreeSet;

use crate::{check::Check, entities::Alias};

pub async fn update_account_build_checks(
    alias: &Alias,
    sources: &BTreeSet<Alias>,
    threshold: u64,
) -> Vec<Check> {
    vec![Check::AccountExist(alias.clone(), threshold, sources.clone())]
}
