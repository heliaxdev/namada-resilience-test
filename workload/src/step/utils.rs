use rand::distributions::uniform::SampleUniform;
use rand::distributions::{Alphanumeric, DistString};
use rand::prelude::IteratorRandom;
use rand::Rng;

use crate::constants::DEFAULT_FEE;
use crate::state::State;
use crate::types::Alias;
use crate::utils::with_rng;

pub(crate) fn coin_flip(p: f64) -> bool {
    with_rng(|rng| rng.gen_bool(p))
}

pub(crate) fn random_between<T: SampleUniform + std::cmp::PartialOrd>(from: T, to: T) -> T {
    if from == to {
        from
    } else {
        with_rng(|rng| rng.gen_range(from..=to))
    }
}

pub(crate) fn random_alias() -> Alias {
    format!(
        "workload-generator-{}",
        with_rng(|rng| Alphanumeric.sample_string(rng, 8))
    )
    .into()
}

pub fn get_random_string(length: usize) -> String {
    let mut result = String::new();
    for _ in 0..length {
        let c = with_rng(|rng| rng.gen_range(0..62));
        let c = if c < 26 {
            (b'a' + c) as char
        } else if c < 52 {
            (b'A' + c - 26) as char
        } else {
            (b'0' + c - 52) as char
        };
        result.push(c);
    }
    result
}

pub fn get_gas_payer<'a>(candidates: impl IntoIterator<Item = &'a Alias>, state: &State) -> Alias {
    let payer = with_rng(|rng| {
        candidates
            .into_iter()
            .filter(|alias| state.get_balance_for(alias) >= DEFAULT_FEE)
            .choose(rng)
            .cloned()
            .unwrap_or(Alias::faucet())
    });

    tracing::info!("Gas payer is {}", payer.name);

    payer
}
