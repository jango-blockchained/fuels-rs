use std::iter::repeat;

use fuel_crypto::Signature;
use fuel_tx::{
    AssetId, Chargeable, ConsensusParameters, Input as FuelInput, TxPointer, Witness,
    field::{Inputs, Outputs, ScriptGasLimit, WitnessLimit, Witnesses},
    input::coin::{CoinPredicate, CoinSigned},
};
use itertools::Itertools;

use crate::{
    constants::WITNESS_STATIC_SIZE,
    types::{DryRun, DryRunner, errors::Result},
};

pub(crate) struct ScriptTxEstimator<R> {
    dry_runner: R,
    predefined_witnesses: Vec<Witness>,
    num_unresolved_witnesses: usize,
    last_dry_run: Option<DryRun>,
}

impl<R> ScriptTxEstimator<R> {
    pub fn new(
        dry_runner: R,
        predefined_witnesses: Vec<Witness>,
        num_unresolved_witnesses: usize,
    ) -> Self {
        Self {
            dry_runner,
            predefined_witnesses,
            num_unresolved_witnesses,
            last_dry_run: None,
        }
    }
}

impl<R: DryRunner> ScriptTxEstimator<R> {
    pub async fn run(
        &mut self,
        mut tx: fuel_tx::Script,
        saturate_variable_outputs: bool,
    ) -> Result<DryRun> {
        self.prepare_for_estimation(&mut tx, saturate_variable_outputs)
            .await?;

        self._run(tx).await
    }

    pub async fn prepare_for_estimation(
        &mut self,
        tx: &mut fuel_tx::Script,
        saturate_variable_outputs: bool,
    ) -> Result<()> {
        let consensus_params = self.dry_runner.consensus_parameters().await?;
        self.add_fake_witnesses(tx);
        self.add_fake_coins(tx, &consensus_params);
        if saturate_variable_outputs {
            self.saturate_with_variable_outputs(tx, &consensus_params);
        }
        self.set_script_gas_limit_to_max(tx, &consensus_params);

        Ok(())
    }

    pub fn last_dry_run(&self) -> Option<DryRun> {
        self.last_dry_run
    }

    async fn _run(&mut self, tx: fuel_tx::Script) -> Result<DryRun> {
        let dry_run = self.dry_runner.dry_run(tx.clone().into()).await?;
        self.last_dry_run = Some(dry_run);

        Ok(dry_run)
    }

    fn set_script_gas_limit_to_max(
        &self,
        tx: &mut fuel_tx::Script,
        consensus_params: &ConsensusParameters,
    ) {
        let max_gas = tx.max_gas(consensus_params.gas_costs(), consensus_params.fee_params()) + 1;
        *tx.script_gas_limit_mut() = consensus_params.tx_params().max_gas_per_tx() - max_gas;
    }

    fn saturate_with_variable_outputs(
        &self,
        tx: &mut fuel_tx::Script,
        consensus_params: &ConsensusParameters,
    ) {
        let max_outputs = usize::from(consensus_params.tx_params().max_outputs());
        let used_outputs = tx.outputs().len();

        let unused_outputs = max_outputs.saturating_sub(used_outputs);

        super::add_variable_outputs(tx, unused_outputs);
    }

    // When dry running a tx with `utxo_validation` off, the node will not validate signatures.
    // However, the node will check if the right number of witnesses is present.
    // This function will create witnesses from a default `Signature` such that the total length matches the expected one.
    // Using a `Signature` ensures that the calculated fee includes the fee generated by the witnesses.
    fn add_fake_witnesses(&self, tx: &mut fuel_tx::Script) {
        let witness: Witness = Signature::default().as_ref().into();
        let dry_run_witnesses: Vec<_> = repeat(witness)
            .take(self.num_unresolved_witnesses)
            .collect();

        *tx.witnesses_mut() = [self.predefined_witnesses.clone(), dry_run_witnesses].concat();
    }

    fn add_fake_coins(&self, tx: &mut fuel_tx::Script, consensus_params: &ConsensusParameters) {
        if let Some(fake_input) =
            Self::needs_fake_base_input(tx.inputs(), consensus_params.base_asset_id())
        {
            tx.inputs_mut().push(fake_input);

            // Add an empty `Witness` for the `coin_signed` we just added
            tx.witnesses_mut().push(Witness::default());
            tx.set_witness_limit(tx.witness_limit() + WITNESS_STATIC_SIZE as u64);
        }
    }

    fn needs_fake_base_input(
        inputs: &[FuelInput],
        base_asset_id: &AssetId,
    ) -> Option<fuel_tx::Input> {
        let has_base_asset = inputs.iter().any(|i| match i {
            FuelInput::CoinSigned(CoinSigned { asset_id, .. })
            | FuelInput::CoinPredicate(CoinPredicate { asset_id, .. })
                if asset_id == base_asset_id =>
            {
                true
            }
            FuelInput::MessageCoinSigned(_) | FuelInput::MessageCoinPredicate(_) => true,
            _ => false,
        });

        if has_base_asset {
            return None;
        }

        let unique_owners = inputs
            .iter()
            .filter_map(|input| match input {
                FuelInput::CoinSigned(CoinSigned { owner, .. })
                | FuelInput::CoinPredicate(CoinPredicate { owner, .. }) => Some(owner),
                _ => None,
            })
            .unique()
            .collect::<Vec<_>>();

        let fake_owner = if let [single_owner] = unique_owners.as_slice() {
            **single_owner
        } else {
            Default::default()
        };

        Some(FuelInput::coin_signed(
            Default::default(),
            fake_owner,
            1_000_000_000,
            *base_asset_id,
            TxPointer::default(),
            0,
        ))
    }
}
