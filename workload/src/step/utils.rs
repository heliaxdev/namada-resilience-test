use antithesis_sdk::random::AntithesisRng;
use rand::distributions::uniform::SampleUniform;
use rand::distributions::{Alphanumeric, DistString};
use rand::prelude::IteratorRandom;
use rand::Rng;

use crate::constants::DEFAULT_FEE;
use crate::state::State;
use crate::types::Alias;

pub(crate) fn coin_flip(p: f64) -> bool {
    AntithesisRng.gen_bool(p)
}

pub(crate) fn random_between<T: SampleUniform + std::cmp::PartialOrd>(from: T, to: T) -> T {
    if from == to {
        from
    } else {
        AntithesisRng.gen_range(from..=to)
    }
}

pub(crate) fn random_alias() -> Alias {
    format!(
        "workload-generator-{}",
        Alphanumeric.sample_string(&mut AntithesisRng, 8)
    )
    .into()
}

pub fn get_random_string(length: usize) -> String {
    let mut result = String::new();
    for _ in 0..length {
        let c = AntithesisRng.gen_range(0..62);
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
    let payer = candidates
        .into_iter()
        .filter(|alias| state.get_balance_for(alias) >= DEFAULT_FEE)
        .choose(&mut AntithesisRng)
        .cloned()
        .unwrap_or(Alias::faucet());

    tracing::info!("Gas payer is {}", payer.name);

    payer
}

#[macro_export]
macro_rules! assert_always_step {
    ($msg:literal, $code:expr) => {
        antithesis_sdk::assert_always_or_unreachable!(true, $msg, &$code.details())
    };
}

#[macro_export]
macro_rules! assert_sometimes_step {
    ($msg:literal, $code:expr) => {
        antithesis_sdk::assert_always_or_unreachable!(true, $msg, &$code.details())
    };
}

#[macro_export]
macro_rules! assert_unreachable_step {
    ($msg:literal, $code:expr) => {
        antithesis_sdk::assert_unreachable!($msg, &$code.details())
    };
}
