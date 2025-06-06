use std::{collections::HashMap, fmt::Debug};

use fuels_core::{
    constants::DEFAULT_CALL_PARAMS_AMOUNT,
    error,
    types::{
        Address, AssetId, ContractId, Selector, errors::Result, input::Input, output::Output,
        param_types::ParamType,
    },
};

use crate::{assembly::contract_call::ContractCallData, calls::utils::sealed};

#[derive(Debug, Clone)]
/// Contains all data relevant to a single contract call
pub struct ContractCall {
    pub contract_id: ContractId,
    pub encoded_args: Result<Vec<u8>>,
    pub encoded_selector: Selector,
    pub call_parameters: CallParameters,
    pub external_contracts: Vec<ContractId>,
    pub output_param: ParamType,
    pub is_payable: bool,
    pub custom_assets: HashMap<(AssetId, Option<Address>), u64>,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

impl ContractCall {
    pub(crate) fn data(&self, base_asset_id: AssetId) -> Result<ContractCallData> {
        let encoded_args = self
            .encoded_args
            .as_ref()
            .map_err(|e| error!(Codec, "cannot encode contract call arguments: {e}"))?
            .to_owned();

        Ok(ContractCallData {
            amount: self.call_parameters.amount(),
            asset_id: self.call_parameters.asset_id().unwrap_or(base_asset_id),
            contract_id: self.contract_id,
            fn_selector_encoded: self.encoded_selector.clone(),
            encoded_args,
            gas_forwarded: self.call_parameters.gas_forwarded,
        })
    }

    pub fn with_contract_id(self, contract_id: ContractId) -> Self {
        ContractCall {
            contract_id,
            ..self
        }
    }

    pub fn with_call_parameters(self, call_parameters: CallParameters) -> ContractCall {
        ContractCall {
            call_parameters,
            ..self
        }
    }

    pub fn add_custom_asset(&mut self, asset_id: AssetId, amount: u64, to: Option<Address>) {
        *self.custom_assets.entry((asset_id, to)).or_default() += amount;
    }

    /// Add custom outputs to the `ContractCall`.
    pub fn with_outputs(mut self, outputs: Vec<Output>) -> Self {
        self.outputs = outputs;
        self
    }

    /// Add custom inputs to the `ContractCall`.
    pub fn with_inputs(mut self, inputs: Vec<Input>) -> Self {
        self.inputs = inputs;
        self
    }
}

impl sealed::Sealed for ContractCall {}

#[derive(Debug, Clone)]
pub struct CallParameters {
    amount: u64,
    asset_id: Option<AssetId>,
    gas_forwarded: Option<u64>,
}

impl CallParameters {
    pub fn new(amount: u64, asset_id: AssetId, gas_forwarded: u64) -> Self {
        Self {
            amount,
            asset_id: Some(asset_id),
            gas_forwarded: Some(gas_forwarded),
        }
    }

    pub fn with_amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn with_asset_id(mut self, asset_id: AssetId) -> Self {
        self.asset_id = Some(asset_id);
        self
    }

    pub fn asset_id(&self) -> Option<AssetId> {
        self.asset_id
    }

    pub fn with_gas_forwarded(mut self, gas_forwarded: u64) -> Self {
        self.gas_forwarded = Some(gas_forwarded);
        self
    }

    pub fn gas_forwarded(&self) -> Option<u64> {
        self.gas_forwarded
    }
}

impl Default for CallParameters {
    fn default() -> Self {
        Self {
            amount: DEFAULT_CALL_PARAMS_AMOUNT,
            asset_id: None,
            gas_forwarded: None,
        }
    }
}
