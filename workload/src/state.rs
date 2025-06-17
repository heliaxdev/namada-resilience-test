use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use rand::seq::IteratorRandom;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::constants::{MAX_BATCH_TX_NUM, PIPELINE_LEN};
use crate::types::{Alias, Epoch, ProposalId};
use crate::utils::with_rng;

#[derive(Error, Debug)]
pub enum StateError {
    #[error("File error: `{0}`")]
    File(std::io::Error),
    #[error("Encode/Decode error: `{0}`")]
    Serde(serde_json::Error),
    #[error("State file is empty")]
    EmptyFile,
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressType {
    Established,
    #[default]
    Implicit,
}

impl AddressType {
    pub fn is_implicit(&self) -> bool {
        matches!(self, AddressType::Implicit)
    }
    pub fn is_established(&self) -> bool {
        matches!(self, AddressType::Established)
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub alias: Alias,
    pub public_keys: BTreeSet<Alias>,
    pub threshold: u64,
    pub address_type: AddressType,
}

impl Account {
    pub fn is_implicit(&self) -> bool {
        self.address_type.is_implicit()
    }
    pub fn is_established(&self) -> bool {
        self.address_type.is_established()
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bond {
    pub alias: Alias,
    pub validator: String,
    pub amount: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub accounts: HashMap<Alias, Account>,
    pub balances: HashMap<Alias, u64>,
    pub masp_balances: HashMap<Alias, u64>,
    pub ibc_balances: HashMap<Alias, HashMap<String, u64>>,
    pub ibc_masp_balances: HashMap<Alias, HashMap<String, u64>>,
    pub foreign_balances: HashMap<Alias, u64>,
    pub bonds: HashMap<Alias, HashMap<String, (u64, Epoch)>>,
    pub unbonds: HashMap<Alias, HashMap<String, u64>>,
    pub redelegations: HashMap<Alias, HashMap<String, u64>>,
    pub claimed_epochs: HashMap<Alias, Epoch>,
    pub validators: HashMap<Alias, Account>,
    pub deactivated_validators: HashMap<Alias, (Account, Epoch)>,
    pub proposals: HashMap<u64, (u64, u64)>,
}

impl State {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::default(),
            balances: HashMap::default(),
            masp_balances: HashMap::default(),
            ibc_balances: HashMap::default(),
            ibc_masp_balances: HashMap::default(),
            foreign_balances: HashMap::default(),
            bonds: HashMap::default(),
            unbonds: HashMap::default(),
            redelegations: HashMap::default(),
            claimed_epochs: HashMap::default(),
            validators: HashMap::default(),
            deactivated_validators: HashMap::default(),
            proposals: HashMap::default(),
        }
    }

    // FILE

    pub fn save(&self, dir: &Path) -> Result<(), StateError> {
        let path = dir.join("state.json");
        let json = serde_json::to_string(self).map_err(StateError::Serde)?;
        std::fs::write(path, json).map_err(StateError::File)
    }

    pub fn load(dir: &Path) -> Result<Self, StateError> {
        let path = dir.join("state.json");
        let json = std::fs::read_to_string(path).map_err(StateError::File)?;
        let state = serde_json::from_str(&json).map_err(StateError::Serde)?;
        Ok(state)
    }

    // READ

    pub fn at_least_masp_account_with_minimal_balance(
        &self,
        number_of_accounts: usize,
        min_balance: u64,
    ) -> bool {
        self.masp_balances
            .iter()
            .filter(|(_, balance)| **balance >= min_balance)
            .count()
            >= number_of_accounts
    }

    pub fn at_least_account_with_min_balance(&self, sample: usize, min_balance: u64) -> bool {
        self.balances
            .iter()
            .filter(|(_, balance)| **balance >= min_balance)
            .count()
            >= sample
    }

    pub fn at_least_bond(&self, sample: usize) -> bool {
        self.bonds
            .values()
            .filter(|data| data.values().any(|(amount, _)| *amount > 2))
            .flatten()
            .count()
            >= sample
    }

    pub fn at_least_validator(&self, sample: usize) -> bool {
        self.validators.len() >= sample
    }

    pub fn at_least_deactivated_validator(&self, sample: usize) -> bool {
        self.deactivated_validators.len() >= sample
    }

    pub fn any_votable_proposal(&self, current_epoch: u64) -> bool {
        self.proposals.iter().any(|(_, (start_epoch, end_epoch))| {
            current_epoch >= *start_epoch && current_epoch < *end_epoch
        })
    }

    // GET

    pub fn random_account(&self, blacklist: Vec<Alias>) -> Option<Account> {
        with_rng(|rng| {
            self.accounts
                .iter()
                .filter(|(alias, _)| !blacklist.contains(alias))
                .choose(rng)
                .map(|(_, account)| account.clone())
        })
    }

    pub fn random_masp_account_with_min_balance(
        &self,
        blacklist: Vec<Alias>,
        min_value: u64,
    ) -> Option<Account> {
        with_rng(|rng| {
            self.masp_balances
                .iter()
                .filter(|(alias, balance)| !blacklist.contains(alias) && **balance >= min_value)
                .filter_map(|(alias, _)| self.accounts.get(alias).cloned())
                .choose(rng)
        })
    }

    pub fn random_account_with_ibc_balance(&self, blacklist: Vec<Alias>) -> Option<Account> {
        with_rng(|rng| {
            self.accounts
                .iter()
                .filter(|(alias, _)| !blacklist.contains(alias))
                .filter(|(alias, _)| {
                    self.ibc_balances
                        .get(alias)
                        .is_some_and(|balances| balances.iter().any(|(_, b)| *b > MAX_BATCH_TX_NUM))
                })
                .map(|(_, account)| account.clone())
                .choose(rng)
        })
    }

    pub fn random_masp_account_with_ibc_balance(&self, blacklist: Vec<Alias>) -> Option<Account> {
        with_rng(|rng| {
            self.accounts
                .iter()
                .filter(|(alias, _)| !blacklist.contains(alias))
                .filter(|(alias, _)| {
                    self.ibc_masp_balances
                        .get(alias)
                        .is_some_and(|balances| balances.iter().any(|(_, b)| *b > MAX_BATCH_TX_NUM))
                })
                .map(|(_, account)| account.clone())
                .choose(rng)
        })
    }

    pub fn random_payment_address(&self, blacklist: Vec<Alias>) -> Option<Account> {
        self.random_implicit_accounts(blacklist, 1).first().cloned()
    }

    pub fn random_implicit_accounts(
        &self,
        blacklist: Vec<Alias>,
        sample_size: usize,
    ) -> Vec<Account> {
        with_rng(|rng| {
            self.accounts
                .iter()
                .filter(|(alias, account)| account.is_implicit() && !blacklist.contains(alias))
                .choose_multiple(rng, sample_size)
                .into_iter()
                .map(|(_, account)| account.clone())
                .collect()
        })
    }

    pub fn random_established_account(
        &self,
        blacklist: Vec<Alias>,
        sample_size: usize,
    ) -> Vec<Account> {
        with_rng(|rng| {
            self.accounts
                .iter()
                .filter(|(alias, _)| !blacklist.contains(alias))
                .filter(|(_, account)| account.is_established())
                .choose_multiple(rng, sample_size)
                .into_iter()
                .map(|(_, account)| account.clone())
                .collect()
        })
    }

    pub fn random_validator(&self, blacklist: Vec<Alias>, sample_size: usize) -> Vec<Account> {
        with_rng(|rng| {
            self.validators
                .iter()
                .filter(|(alias, _)| !blacklist.contains(alias))
                .filter(|(_, account)| account.is_established())
                .choose_multiple(rng, sample_size)
                .into_iter()
                .map(|(_, account)| account.clone())
                .collect()
        })
    }

    pub fn random_deactivated_validator(
        &self,
        blacklist: Vec<Alias>,
        current_epoch: Epoch,
        sample_size: usize,
    ) -> Vec<Account> {
        with_rng(|rng| {
            self.deactivated_validators
                .iter()
                .filter(|(alias, (account, epoch))| {
                    !blacklist.contains(alias)
                        && account.is_established()
                        && current_epoch > epoch + PIPELINE_LEN
                })
                .choose_multiple(rng, sample_size)
                .into_iter()
                .map(|(_, (account, _))| account.clone())
                .collect()
        })
    }

    pub fn random_bond(&self, current_epoch: Epoch) -> Option<Bond> {
        with_rng(|rng| {
            self.bonds
                .iter()
                .flat_map(|(source, bonds)| {
                    bonds.iter().filter_map(|(validator, (amount, epoch))| {
                        // the bond was requested at the epoch,
                        // but the execution could be at the next epoch
                        if *amount > 0 && current_epoch > epoch + PIPELINE_LEN {
                            Some(Bond {
                                alias: source.to_owned(),
                                validator: validator.to_owned(),
                                amount: *amount,
                            })
                        } else {
                            None
                        }
                    })
                })
                .choose(rng)
        })
    }

    pub fn random_account_with_min_balance(
        &self,
        blacklist: Vec<Alias>,
        min_balance: u64,
    ) -> Option<Account> {
        with_rng(|rng| {
            self.balances
                .iter()
                .filter_map(|(alias, balance)| {
                    if blacklist.contains(alias) {
                        return None;
                    }
                    if balance >= &min_balance {
                        Some(self.accounts.get(alias).unwrap().clone())
                    } else {
                        None
                    }
                })
                .choose(rng)
        })
    }

    pub fn get_account_by_alias(&self, alias: &Alias) -> Account {
        self.accounts.get(alias).unwrap().to_owned()
    }

    pub fn get_claimed_epoch(&self, alias: &Alias) -> Option<Epoch> {
        self.claimed_epochs.get(alias).cloned()
    }

    pub fn get_balance_for(&self, alias: &Alias) -> u64 {
        self.balances.get(alias).cloned().unwrap_or_default()
    }

    pub fn get_shielded_balance_for(&self, alias: &Alias) -> u64 {
        self.masp_balances
            .get(&alias.base())
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_ibc_balance_for(&self, alias: &Alias, denom: &str) -> u64 {
        let balances = if alias.is_spending_key() || alias.is_payment_address() {
            self.ibc_masp_balances.get(&alias.base())
        } else {
            self.ibc_balances.get(alias)
        };
        let Some(balances) = balances else {
            return 0;
        };
        balances.get(denom).cloned().unwrap_or_default()
    }

    pub fn get_foreign_balance_for(&self, alias: &Alias) -> u64 {
        self.foreign_balances
            .get(alias)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_redelegations_targets_for(&self, alias: &Alias) -> HashSet<String> {
        self.redelegations
            .get(alias)
            .map(|data| data.keys().cloned().collect::<HashSet<String>>())
            .unwrap_or_default()
    }

    pub fn random_votable_proposal(&self, current_epoch: u64) -> Option<u64> {
        with_rng(|rng| {
            self.proposals
                .iter()
                .filter_map(|(proposal_id, (start_epoch, end_epoch))| {
                    // the following vote will be request at the current epoch,
                    // but the execution could be at the next epoch
                    if current_epoch >= *start_epoch && current_epoch < *end_epoch - 1 {
                        Some(proposal_id.to_owned())
                    } else {
                        None
                    }
                })
                .choose(rng)
        })
    }

    // UPDATE

    pub fn add_implicit_account(&mut self, alias: &Alias) {
        self.accounts.insert(
            alias.clone(),
            Account {
                alias: alias.clone(),
                public_keys: BTreeSet::from_iter(vec![alias.clone()]),
                threshold: 1,
                address_type: AddressType::Implicit,
            },
        );
        self.balances.insert(alias.clone(), 0);
        self.masp_balances.insert(alias.clone(), 0);
    }

    pub fn add_established_account(
        &mut self,
        alias: &Alias,
        aliases: &BTreeSet<Alias>,
        threshold: u64,
    ) {
        self.accounts.insert(
            alias.clone(),
            Account {
                alias: alias.clone(),
                public_keys: aliases.clone(),
                threshold,
                address_type: AddressType::Established,
            },
        );
        self.balances.insert(alias.clone(), 0);
    }

    pub fn modify_established_account(
        &mut self,
        alias: &Alias,
        aliases: &BTreeSet<Alias>,
        threshold: u64,
    ) {
        self.accounts.entry(alias.clone()).and_modify(|account| {
            account.public_keys = aliases.clone();
            account.threshold = threshold;
        });
    }

    pub fn increase_balance(&mut self, target: &Alias, amount: u64) {
        if target.is_faucet() {
            return;
        }
        *self.balances.get_mut(target).unwrap() += amount;
    }

    pub fn increase_masp_balance(&mut self, target: &Alias, amount: u64) {
        *self.masp_balances.get_mut(&target.base()).unwrap() += amount;
    }

    pub fn increase_ibc_balance(&mut self, target: &Alias, denom: &str, amount: u64) {
        if target.is_faucet() {
            return;
        }
        let default = HashMap::from_iter([(denom.to_string(), 0)]);
        if target.is_spending_key() || target.is_payment_address() {
            *self
                .ibc_masp_balances
                .entry(target.base())
                .or_insert(default)
                .entry(denom.to_string())
                .or_insert(0) += amount;
        } else {
            *self
                .ibc_balances
                .entry(target.clone())
                .or_insert(default)
                .entry(denom.to_string())
                .or_insert(0) += amount;
        }
    }

    pub fn increase_foreign_balance(&mut self, target: &Alias, amount: u64) {
        *self.foreign_balances.entry(target.clone()).or_insert(0) += amount;
    }

    pub fn decrease_balance(&mut self, target: &Alias, amount: u64) {
        if target.is_faucet() {
            return;
        }
        *self.balances.get_mut(target).unwrap() -= amount;
    }

    pub fn decrease_masp_balance(&mut self, target: &Alias, amount: u64) {
        *self.masp_balances.get_mut(&target.base()).unwrap() -= amount;
    }

    pub fn decrease_ibc_balance(&mut self, target: &Alias, denom: &str, amount: u64) {
        if target.is_faucet() {
            return;
        }
        if target.is_spending_key() || target.is_payment_address() {
            self.ibc_masp_balances
                .entry(target.base())
                .and_modify(|balance| *balance.get_mut(denom).unwrap() -= amount);
        } else {
            self.ibc_balances
                .entry(target.clone())
                .and_modify(|balance| *balance.get_mut(denom).unwrap() -= amount);
        }
    }

    pub fn decrease_foreign_balance(&mut self, target: &Alias, amount: u64) {
        *self.foreign_balances.get_mut(target).unwrap() -= amount;
    }

    pub fn modify_balance_fee(&mut self, source: &Alias, fee: u64) {
        if source.is_spending_key() {
            *self.masp_balances.get_mut(&source.base()).unwrap() -= fee;
        } else if !source.is_faucet() {
            *self.balances.get_mut(source).unwrap() -= fee;
        }
    }

    pub fn modify_bond(&mut self, source: &Alias, validator: &str, amount: u64, epoch: Epoch) {
        if !source.is_faucet() {
            *self.balances.get_mut(source).unwrap() -= amount;
        }
        let default = HashMap::from_iter([(validator.to_string(), (0u64, 0u64))]);
        let bond = self
            .bonds
            .entry(source.clone())
            .or_insert(default)
            .entry(validator.to_string())
            .or_insert((0, 0));
        bond.0 += amount;
        bond.1 = epoch;
    }

    pub fn modify_redelegate(&mut self, source: &Alias, from: &str, to: &str, amount: u64) {
        let default = HashMap::from_iter([(to.to_string(), 0u64)]);
        *self
            .redelegations
            .entry(source.clone())
            .or_insert(default)
            .entry(to.to_string())
            .or_insert(0) += amount;
        self.bonds
            .entry(source.clone())
            .and_modify(|bond| bond.get_mut(from).unwrap().0 -= amount);
    }

    pub fn modify_unbond(&mut self, source: &Alias, validator: &str, amount: u64) {
        let default = HashMap::from_iter([(validator.to_string(), 0u64)]);
        *self
            .unbonds
            .entry(source.clone())
            .or_insert(default)
            .entry(validator.to_string())
            .or_insert(0) += amount;
        self.bonds
            .entry(source.clone())
            .and_modify(|bond| bond.get_mut(validator).unwrap().0 -= amount);
    }

    pub fn modify_shielding(&mut self, source: &Alias, target: &Alias, amount: u64) {
        *self.balances.get_mut(source).unwrap() -= amount;
        *self.masp_balances.get_mut(&target.base()).unwrap() += amount;
    }

    pub fn modify_unshielding(&mut self, source: &Alias, target: &Alias, amount: u64) {
        *self.masp_balances.get_mut(&source.base()).unwrap() -= amount;
        *self.balances.get_mut(target).unwrap() += amount;
    }

    pub fn modify_shielded_transfer(&mut self, source: &Alias, target: &Alias, amount: u64) {
        *self.masp_balances.get_mut(&target.base()).unwrap() += amount;
        *self.masp_balances.get_mut(&source.base()).unwrap() -= amount;
    }

    pub fn set_established_as_validator(&mut self, alias: &Alias) {
        let account = self.accounts.remove(alias).unwrap();
        self.balances.remove(alias).unwrap();
        self.validators.insert(alias.clone(), account);
    }

    pub fn set_validator_as_deactivated(&mut self, alias: &Alias, epoch: Epoch) {
        let account = self.validators.remove(alias).unwrap();
        self.deactivated_validators
            .insert(alias.clone(), (account, epoch));
    }

    pub fn reactivate_validator(&mut self, alias: &Alias) {
        let (account, _) = self.deactivated_validators.remove(alias).unwrap();
        self.validators.insert(alias.clone(), account);
    }

    pub fn add_proposals(&mut self, new_proposals: HashMap<ProposalId, (Epoch, Epoch)>) {
        self.proposals.extend(new_proposals);
    }

    pub fn set_claimed_epoch(&mut self, source: &Alias, epoch: Epoch) {
        let claimed_epoch = self.claimed_epochs.entry(source.clone()).or_insert(0);
        if epoch > *claimed_epoch {
            *claimed_epoch = epoch;
        }
    }

    pub fn overwrite_balance(&mut self, source: &Alias, balance: u64) {
        *self.balances.get_mut(source).unwrap() = balance;
    }
}
