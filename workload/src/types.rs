use std::fmt;

use namada_sdk::dec::Dec;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Alias {
    pub name: String,
}

impl From<String> for Alias {
    fn from(value: String) -> Self {
        Alias { name: value }
    }
}

impl From<&str> for Alias {
    fn from(value: &str) -> Self {
        Alias {
            name: value.to_string(),
        }
    }
}

impl Alias {
    pub fn faucet() -> Self {
        Self {
            name: "faucet".into(),
        }
    }

    pub fn nam() -> Self {
        Self { name: "nam".into() }
    }

    pub fn is_faucet(&self) -> bool {
        self.eq(&Self::faucet())
    }

    pub fn base(&self) -> Self {
        let name = if self.name.ends_with("-spending-key") {
            self.name
                .strip_suffix("-spending-key")
                .expect("the suffix should exist")
        } else if self.name.ends_with("-payment-address") {
            self.name
                .strip_suffix("-payment-address")
                .expect("the suffix should exist")
        } else {
            &self.name
        }
        .to_string();

        Self { name }
    }

    pub fn established(&self) -> Self {
        let name = format!("{}-established", self.name);

        Self { name }
    }

    pub fn spending_key(&self) -> Self {
        let name = format!("{}-spending-key", self.base().name);

        Self { name }
    }

    pub fn payment_address(&self) -> Self {
        let name = format!("{}-payment-address", self.base().name);

        Self { name }
    }

    pub fn is_spending_key(&self) -> bool {
        self.name.ends_with("-spending-key")
    }
}

impl Serialize for Alias {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Serialize::serialize(&self.name, serializer)
    }
}

impl<'de> Deserialize<'de> for Alias {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AliasVisitor;

        impl Visitor<'_> for AliasVisitor {
            type Value = Alias;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing the Alias name")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Alias {
                    name: value.to_string(),
                })
            }
        }

        deserializer.deserialize_str(AliasVisitor)
    }
}

#[derive(Clone, Debug)]
pub enum ValidatorStatus {
    Active,
    Reactivating,
    Inactive,
}

impl fmt::Display for ValidatorStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidatorStatus::Active => write!(f, "active"),
            ValidatorStatus::Inactive => write!(f, "inactive"),
            ValidatorStatus::Reactivating => write!(f, "reactivating"),
        }
    }
}

pub type Amount = u64;
pub type ValidatorAddress = String;
pub type Epoch = u64;
pub type MaspEpoch = namada_sdk::token::MaspEpoch;
pub type Threshold = u64;
pub type CommissionRate = Dec;
pub type CommissionChange = Dec;
pub type ProposalId = u64;
pub type ProposalVote = namada_sdk::governance::ProposalVote;
pub type Height = u64;
pub type Balance = namada_sdk::token::Amount;
pub type Fee = u64;
