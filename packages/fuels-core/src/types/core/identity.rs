use fuel_types::{Address, ContractId};
use fuels_macros::{Parameterize, Tokenizable, TryFrom};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Parameterize,
    Tokenizable,
    TryFrom,
    Serialize,
    Deserialize,
)]
#[FuelsCorePath = "crate"]
#[FuelsTypesPath = "crate::types"]
pub enum Identity {
    Address(Address),
    ContractId(ContractId),
}

impl Default for Identity {
    fn default() -> Self {
        Self::Address(Address::default())
    }
}

impl AsRef<[u8]> for Identity {
    fn as_ref(&self) -> &[u8] {
        match self {
            Identity::Address(address) => address.as_ref(),
            Identity::ContractId(contract_id) => contract_id.as_ref(),
        }
    }
}

impl From<&Address> for Identity {
    fn from(address: &Address) -> Self {
        Self::Address(*address)
    }
}
impl From<Address> for Identity {
    fn from(address: Address) -> Self {
        Self::Address(address)
    }
}

impl From<&ContractId> for Identity {
    fn from(contract_id: &ContractId) -> Self {
        Self::ContractId(*contract_id)
    }
}
impl From<ContractId> for Identity {
    fn from(contract_id: ContractId) -> Self {
        Self::ContractId(contract_id)
    }
}
