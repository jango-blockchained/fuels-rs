use crate::{provider::Provider, signers::private_key::PrivateKeySigner};

#[derive(Debug, Clone)]
pub struct Wallet<S = Unlocked<PrivateKeySigner>> {
    state: S,
    provider: Provider,
}

impl<S> Wallet<S> {
    pub fn set_provider(&mut self, provider: Provider) {
        self.provider = provider;
    }

    pub fn provider(&self) -> &Provider {
        &self.provider
    }
}

mod unlocked {
    use async_trait::async_trait;
    use fuels_core::{
        traits::Signer,
        types::{
            Address, AssetId, coin_type_id::CoinTypeId, errors::Result, input::Input,
            transaction_builders::TransactionBuilder,
        },
    };
    use rand::{CryptoRng, RngCore};

    use super::{Locked, Wallet};
    use crate::{
        Account, ViewOnlyAccount, provider::Provider, signers::private_key::PrivateKeySigner,
    };

    #[derive(Debug, Clone)]
    pub struct Unlocked<S> {
        signer: S,
    }

    impl<S> Unlocked<S> {
        fn new(signer: S) -> Self {
            Self { signer }
        }
    }

    impl<S> Wallet<Unlocked<S>> {
        pub fn new(signer: S, provider: Provider) -> Self {
            Wallet {
                state: Unlocked::new(signer),
                provider,
            }
        }

        pub fn signer(&self) -> &S {
            &self.state.signer
        }
    }

    impl Wallet<Unlocked<PrivateKeySigner>> {
        pub fn random(rng: &mut (impl CryptoRng + RngCore), provider: Provider) -> Self {
            Self::new(PrivateKeySigner::random(rng), provider)
        }
    }

    impl<S> Wallet<Unlocked<S>>
    where
        S: Signer,
    {
        pub fn lock(&self) -> Wallet<Locked> {
            Wallet::new_locked(self.state.signer.address(), self.provider.clone())
        }
    }

    #[async_trait]
    impl<S> ViewOnlyAccount for Wallet<Unlocked<S>>
    where
        S: Signer + Clone + Send + Sync + std::fmt::Debug + 'static,
    {
        fn address(&self) -> Address {
            self.state.signer.address()
        }

        fn try_provider(&self) -> Result<&Provider> {
            Ok(&self.provider)
        }

        async fn get_asset_inputs_for_amount(
            &self,
            asset_id: AssetId,
            amount: u128,
            excluded_coins: Option<Vec<CoinTypeId>>,
        ) -> Result<Vec<Input>> {
            Ok(self
                .get_spendable_resources(asset_id, amount, excluded_coins)
                .await?
                .into_iter()
                .map(Input::resource_signed)
                .collect::<Vec<Input>>())
        }
    }

    #[async_trait]
    impl<S> Account for Wallet<Unlocked<S>>
    where
        S: Signer + Clone + Send + Sync + std::fmt::Debug + 'static,
    {
        fn add_witnesses<Tb: TransactionBuilder>(&self, tb: &mut Tb) -> Result<()> {
            tb.add_signer(self.state.signer.clone())?;

            Ok(())
        }
    }
}
pub use unlocked::*;

mod locked {
    use async_trait::async_trait;
    use fuels_core::types::{
        Address, AssetId, coin_type_id::CoinTypeId, errors::Result, input::Input,
    };

    use super::Wallet;
    use crate::{ViewOnlyAccount, provider::Provider};

    #[derive(Debug, Clone)]
    pub struct Locked {
        address: Address,
    }

    impl Locked {
        fn new(address: Address) -> Self {
            Self { address }
        }
    }

    impl Wallet<Locked> {
        pub fn new_locked(addr: Address, provider: Provider) -> Self {
            Self {
                state: Locked::new(addr),
                provider,
            }
        }
    }

    #[async_trait]
    impl ViewOnlyAccount for Wallet<Locked> {
        fn address(&self) -> Address {
            self.state.address
        }

        fn try_provider(&self) -> Result<&Provider> {
            Ok(&self.provider)
        }

        async fn get_asset_inputs_for_amount(
            &self,
            asset_id: AssetId,
            amount: u128,
            excluded_coins: Option<Vec<CoinTypeId>>,
        ) -> Result<Vec<Input>> {
            Ok(self
                .get_spendable_resources(asset_id, amount, excluded_coins)
                .await?
                .into_iter()
                .map(Input::resource_signed)
                .collect::<Vec<Input>>())
        }
    }
}
pub use locked::*;
