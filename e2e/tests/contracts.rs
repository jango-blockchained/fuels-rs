use std::time::Duration;

use fuel_tx::SubAssetId;
use fuels::{
    accounts::signers::private_key::PrivateKeySigner,
    core::codec::{DecoderConfig, EncoderConfig, calldata, encode_fn_selector},
    prelude::*,
    programs::DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE,
    tx::{
        ConsensusParameters, ContractIdExt, ContractParameters, FeeParameters,
        consensus_parameters::{ConsensusParametersV1, FeeParametersV1},
    },
    types::{
        Bits256, Identity, SizedAsciiString, errors::transaction::Reason, input::Input,
        output::Output,
    },
};
use rand::thread_rng;
use tokio::time::Instant;

#[tokio::test]
async fn test_multiple_args() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    // Make sure we can call the contract with multiple arguments
    let contract_methods = contract_instance.methods();
    let response = contract_methods.get(5, 6).call().await?;

    assert_eq!(response.value, 11);

    let t = MyType { x: 5, y: 6 };
    let response = contract_methods.get_alt(t.clone()).call().await?;
    assert_eq!(response.value, t);

    let response = contract_methods.get_single(5).call().await?;
    assert_eq!(response.value, 5);
    Ok(())
}

#[tokio::test]
async fn test_contract_calling_contract() -> Result<()> {
    // Tests a contract call that calls another contract (FooCaller calls FooContract underneath)
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "LibContract",
                project = "e2e/sway/contracts/lib_contract"
            ),
            Contract(
                name = "LibContractCaller",
                project = "e2e/sway/contracts/lib_contract_caller"
            ),
        ),
        Deploy(
            name = "lib_contract_instance",
            contract = "LibContract",
            wallet = "wallet",
            random_salt = false,
        ),
        Deploy(
            name = "lib_contract_instance2",
            contract = "LibContract",
            wallet = "wallet",
            random_salt = false,
        ),
        Deploy(
            name = "contract_caller_instance",
            contract = "LibContractCaller",
            wallet = "wallet",
            random_salt = false,
        ),
    );
    let lib_contract_id = lib_contract_instance.contract_id();
    let lib_contract_id2 = lib_contract_instance2.contract_id();

    // Call the contract directly. It increments the given value.
    let response = lib_contract_instance.methods().increment(42).call().await?;

    assert_eq!(43, response.value);

    let response = contract_caller_instance
        .methods()
        .increment_from_contracts(lib_contract_id, lib_contract_id2, 42)
        // Note that the two lib_contract_instances have different types
        .with_contracts(&[&lib_contract_instance, &lib_contract_instance2])
        .call()
        .await?;

    assert_eq!(86, response.value);

    // ANCHOR: external_contract
    let response = contract_caller_instance
        .methods()
        .increment_from_contract(lib_contract_id, 42)
        .with_contracts(&[&lib_contract_instance])
        .call()
        .await?;
    // ANCHOR_END: external_contract

    assert_eq!(43, response.value);

    // ANCHOR: external_contract_ids
    let response = contract_caller_instance
        .methods()
        .increment_from_contract(lib_contract_id, 42)
        .with_contract_ids(&[lib_contract_id])
        .call()
        .await?;
    // ANCHOR_END: external_contract_ids

    assert_eq!(43, response.value);
    Ok(())
}

#[tokio::test]
async fn test_reverting_transaction() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "RevertContract",
            project = "e2e/sway/contracts/revert_transaction_error"
        )),
        Deploy(
            name = "contract_instance",
            contract = "RevertContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let response = contract_instance
        .methods()
        .make_transaction_fail(true)
        .call()
        .await;

    assert!(matches!(
        response,
        Err(Error::Transaction(Reason::Failure { revert_id, .. })) if revert_id == Some(128)
    ));

    Ok(())
}

#[tokio::test]
async fn test_multiple_read_calls() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MultiReadContract",
            project = "e2e/sway/contracts/multiple_read_calls"
        )),
        Deploy(
            name = "contract_instance",
            contract = "MultiReadContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let contract_methods = contract_instance.methods();
    contract_methods.store(42).call().await?;

    // Use "simulate" because the methods don't actually
    // run a transaction, but just a dry-run
    let stored = contract_methods
        .read()
        .simulate(Execution::state_read_only())
        .await?;

    assert_eq!(stored.value, 42);

    let stored = contract_methods
        .read()
        .simulate(Execution::state_read_only())
        .await?;

    assert_eq!(stored.value, 42);
    Ok(())
}

#[tokio::test]
async fn test_multi_call_beginner() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let contract_methods = contract_instance.methods();
    let call_handler_1 = contract_methods.get_single(7);
    let call_handler_2 = contract_methods.get_single(42);

    let multi_call_handler = CallHandler::new_multi_call(wallet.clone())
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let (val_1, val_2): (u64, u64) = multi_call_handler.call().await?.value;

    assert_eq!(val_1, 7);
    assert_eq!(val_2, 42);

    Ok(())
}

#[tokio::test]
async fn test_multi_call_pro() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let my_type_1 = MyType { x: 1, y: 2 };
    let my_type_2 = MyType { x: 3, y: 4 };

    let contract_methods = contract_instance.methods();
    let call_handler_1 = contract_methods.get_single(5);
    let call_handler_2 = contract_methods.get_single(6);
    let call_handler_3 = contract_methods.get_alt(my_type_1.clone());
    let call_handler_4 = contract_methods.get_alt(my_type_2.clone());
    let call_handler_5 = contract_methods.get_array([7; 2]);
    let call_handler_6 = contract_methods.get_array([42; 2]);

    let multi_call_handler = CallHandler::new_multi_call(wallet.clone())
        .add_call(call_handler_1)
        .add_call(call_handler_2)
        .add_call(call_handler_3)
        .add_call(call_handler_4)
        .add_call(call_handler_5)
        .add_call(call_handler_6);

    let (val_1, val_2, type_1, type_2, array_1, array_2): (
        u64,
        u64,
        MyType,
        MyType,
        [u64; 2],
        [u64; 2],
    ) = multi_call_handler.call().await?.value;

    assert_eq!(val_1, 5);
    assert_eq!(val_2, 6);
    assert_eq!(type_1, my_type_1);
    assert_eq!(type_2, my_type_2);
    assert_eq!(array_1, [7; 2]);
    assert_eq!(array_2, [42; 2]);

    Ok(())
}

#[tokio::test]
async fn test_contract_call_fee_estimation() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let gas_limit = 800;
    let tolerance = Some(0.2);
    let block_horizon = Some(1);
    let expected_script_gas = 800;
    let expected_total_gas = 8463;
    let expected_metered_bytes_size = 824;

    let estimated_transaction_cost = contract_instance
        .methods()
        .initialize_counter(42)
        .with_tx_policies(TxPolicies::default().with_script_gas_limit(gas_limit))
        .estimate_transaction_cost(tolerance, block_horizon)
        .await?;

    assert_eq!(estimated_transaction_cost.script_gas, expected_script_gas);
    assert_eq!(estimated_transaction_cost.total_gas, expected_total_gas);
    assert_eq!(
        estimated_transaction_cost.metered_bytes_size,
        expected_metered_bytes_size
    );

    Ok(())
}

#[tokio::test]
async fn contract_call_has_same_estimated_and_used_gas() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );
    let contract_methods = contract_instance.methods();

    let tolerance = Some(0.0);
    let block_horizon = Some(1);

    let estimated_total_gas = contract_methods
        .initialize_counter(42)
        .estimate_transaction_cost(tolerance, block_horizon)
        .await?
        .total_gas;

    let gas_used = contract_methods
        .initialize_counter(42)
        .call()
        .await?
        .tx_status
        .total_gas;

    assert_eq!(estimated_total_gas, gas_used);
    Ok(())
}

#[tokio::test]
async fn mult_call_has_same_estimated_and_used_gas() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let contract_methods = contract_instance.methods();
    let call_handler_1 = contract_methods.initialize_counter(42);
    let call_handler_2 = contract_methods.get_array([42; 2]);

    let multi_call_handler = CallHandler::new_multi_call(wallet.clone())
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let tolerance = Some(0.0);
    let block_horizon = Some(1);
    let estimated_total_gas = multi_call_handler
        .estimate_transaction_cost(tolerance, block_horizon)
        .await?
        .total_gas;

    let total_gas = multi_call_handler
        .call::<(u64, [u64; 2])>()
        .await?
        .tx_status
        .total_gas;

    assert_eq!(estimated_total_gas, total_gas);

    Ok(())
}

#[tokio::test]
async fn contract_method_call_respects_maturity_and_expiration() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "BlockHeightContract",
            project = "e2e/sway/contracts/transaction_block_height"
        )),
        Deploy(
            name = "contract_instance",
            contract = "BlockHeightContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );
    let provider = wallet.provider();

    let maturity = 10;
    let expiration = 20;
    let call_handler = contract_instance
        .methods()
        .calling_this_will_produce_a_block()
        .with_tx_policies(
            TxPolicies::default()
                .with_maturity(maturity)
                .with_expiration(expiration),
        );

    {
        let err = call_handler
            .clone()
            .call()
            .await
            .expect_err("maturity not reached");

        assert!(err.to_string().contains("TransactionMaturity"));
    }
    {
        provider.produce_blocks(15, None).await?;
        call_handler
            .clone()
            .call()
            .await
            .expect("should succeed. Block height between `maturity` and `expiration`");
    }
    {
        provider.produce_blocks(15, None).await?;
        let err = call_handler.call().await.expect_err("expiration reached");

        assert!(err.to_string().contains("TransactionExpiration"));
    }

    Ok(())
}

#[tokio::test]
async fn test_auth_msg_sender_from_sdk() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "AuthContract",
            project = "e2e/sway/contracts/auth_testing_contract"
        )),
        Deploy(
            name = "contract_instance",
            contract = "AuthContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    // Contract returns true if `msg_sender()` matches `wallet.address()`.
    let response = contract_instance
        .methods()
        .check_msg_sender(wallet.address())
        .call()
        .await?;

    assert!(response.value);
    Ok(())
}

#[tokio::test]
async fn test_large_return_data() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/large_return_data"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let contract_methods = contract_instance.methods();
    let res = contract_methods.get_id().call().await?;

    assert_eq!(
        res.value.0,
        [
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
        ]
    );

    // One word-sized string
    let res = contract_methods.get_small_string().call().await?;
    assert_eq!(res.value, "gggggggg");

    // Two word-sized string
    let res = contract_methods.get_large_string().call().await?;
    assert_eq!(res.value, "ggggggggg");

    // Large struct will be bigger than a `WORD`.
    let res = contract_methods.get_large_struct().call().await?;
    assert_eq!(res.value.foo, 12);
    assert_eq!(res.value.bar, 42);

    // Array will be returned in `ReturnData`.
    let res = contract_methods.get_large_array().call().await?;
    assert_eq!(res.value, [1, 2]);

    let res = contract_methods.get_contract_id().call().await?;

    // First `value` is from `CallResponse`.
    // Second `value` is from the `ContractId` type.
    assert_eq!(
        res.value,
        ContractId::from([
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255
        ])
    );
    Ok(())
}

#[tokio::test]
async fn can_handle_function_called_new() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let response = contract_instance.methods().new().call().await?.value;

    assert_eq!(response, 12345);
    Ok(())
}

#[tokio::test]
async fn test_contract_setup_macro_deploy_with_salt() -> Result<()> {
    // ANCHOR: contract_setup_macro_multi
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "LibContract",
                project = "e2e/sway/contracts/lib_contract"
            ),
            Contract(
                name = "LibContractCaller",
                project = "e2e/sway/contracts/lib_contract_caller"
            ),
        ),
        Deploy(
            name = "lib_contract_instance",
            contract = "LibContract",
            wallet = "wallet",
            random_salt = false,
        ),
        Deploy(
            name = "contract_caller_instance",
            contract = "LibContractCaller",
            wallet = "wallet",
        ),
        Deploy(
            name = "contract_caller_instance2",
            contract = "LibContractCaller",
            wallet = "wallet",
        ),
    );
    let lib_contract_id = lib_contract_instance.contract_id();

    let contract_caller_id = contract_caller_instance.contract_id();

    let contract_caller_id2 = contract_caller_instance2.contract_id();

    // Because we deploy with salt, we can deploy the same contract multiple times
    assert_ne!(contract_caller_id, contract_caller_id2);

    // The first contract can be called because they were deployed on the same provider
    let response = contract_caller_instance
        .methods()
        .increment_from_contract(lib_contract_id, 42)
        .with_contracts(&[&lib_contract_instance])
        .call()
        .await?;

    assert_eq!(43, response.value);

    let response = contract_caller_instance2
        .methods()
        .increment_from_contract(lib_contract_id, 42)
        .with_contracts(&[&lib_contract_instance])
        .call()
        .await?;

    assert_eq!(43, response.value);
    // ANCHOR_END: contract_setup_macro_multi

    Ok(())
}

#[tokio::test]
async fn test_wallet_getter() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    assert_eq!(contract_instance.account().address(), wallet.address());
    //`contract_id()` is tested in
    // async fn test_contract_calling_contract() -> Result<()> {
    Ok(())
}

#[tokio::test]
async fn test_connect_wallet() -> Result<()> {
    // ANCHOR: contract_setup_macro_manual_wallet
    let config = WalletsConfig::new(Some(2), Some(1), Some(DEFAULT_COIN_AMOUNT));

    let mut wallets = launch_custom_provider_and_get_wallets(config, None, None).await?;
    let wallet = wallets.pop().unwrap();
    let wallet_2 = wallets.pop().unwrap();

    setup_program_test!(
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );
    // ANCHOR_END: contract_setup_macro_manual_wallet

    // pay for call with wallet
    let tx_policies = TxPolicies::default()
        .with_tip(100)
        .with_script_gas_limit(1_000_000);

    contract_instance
        .methods()
        .initialize_counter(42)
        .with_tx_policies(tx_policies)
        .call()
        .await?;

    // confirm that funds have been deducted
    let wallet_balance = wallet.get_asset_balance(&Default::default()).await?;
    assert!(DEFAULT_COIN_AMOUNT as u128 > wallet_balance);

    // pay for call with wallet_2
    contract_instance
        .with_account(wallet_2.clone())
        .methods()
        .initialize_counter(42)
        .with_tx_policies(tx_policies)
        .call()
        .await?;

    // confirm there are no changes to wallet, wallet_2 has been charged
    let wallet_balance_second_call = wallet.get_asset_balance(&Default::default()).await?;
    let wallet_2_balance = wallet_2.get_asset_balance(&Default::default()).await?;
    assert_eq!(wallet_balance_second_call, wallet_balance);
    assert!(DEFAULT_COIN_AMOUNT as u128 > wallet_2_balance);

    Ok(())
}

async fn setup_output_variable_estimation_test()
-> Result<(Vec<Wallet>, [Identity; 3], AssetId, ContractId)> {
    let wallet_config = WalletsConfig::new(Some(3), None, None);
    let wallets = launch_custom_provider_and_get_wallets(wallet_config, None, None).await?;

    let contract_id = Contract::load_from(
        "sway/contracts/token_ops/out/release/token_ops.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&wallets[0], TxPolicies::default())
    .await?
    .contract_id;

    let mint_asset_id = contract_id.asset_id(&SubAssetId::zeroed());
    let addresses = wallets
        .iter()
        .map(|wallet| wallet.address().into())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    Ok((wallets, addresses, mint_asset_id, contract_id))
}

#[tokio::test]
async fn test_output_variable_estimation() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/token_ops/out/release/token_ops-abi.json"
    ));

    let (wallets, addresses, mint_asset_id, contract_id) =
        setup_output_variable_estimation_test().await?;

    let contract_instance = MyContract::new(contract_id, wallets[0].clone());
    let contract_methods = contract_instance.methods();
    let amount = 1000;

    {
        // Should fail due to lack of output variables
        let response = contract_methods
            .mint_to_addresses(amount, addresses)
            .call()
            .await;

        assert!(matches!(
            response,
            Err(Error::Transaction(Reason::Failure { .. }))
        ));
    }

    {
        // Should add 3 output variables automatically
        let _ = contract_methods
            .mint_to_addresses(amount, addresses)
            .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
            .call()
            .await?;

        for wallet in wallets.iter() {
            let balance = wallet.get_asset_balance(&mint_asset_id).await?;
            assert_eq!(balance, amount as u128);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_output_variable_estimation_multicall() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/token_ops/out/release/token_ops-abi.json"
    ));

    let (wallets, addresses, mint_asset_id, contract_id) =
        setup_output_variable_estimation_test().await?;

    let contract_instance = MyContract::new(contract_id, wallets[0].clone());
    let contract_methods = contract_instance.methods();
    const NUM_OF_CALLS: u64 = 3;
    let amount = 1000;
    let total_amount = amount * NUM_OF_CALLS;

    let mut multi_call_handler = CallHandler::new_multi_call(wallets[0].clone());
    for _ in 0..NUM_OF_CALLS {
        let call_handler = contract_methods.mint_to_addresses(amount, addresses);
        multi_call_handler = multi_call_handler.add_call(call_handler);
    }

    wallets[0]
        .force_transfer_to_contract(
            contract_id,
            total_amount,
            AssetId::zeroed(),
            TxPolicies::default(),
        )
        .await
        .unwrap();

    let base_layer_address = Bits256([1u8; 32]);
    let call_handler = contract_methods.send_message(base_layer_address, amount);
    multi_call_handler = multi_call_handler.add_call(call_handler);

    let _ = multi_call_handler
        .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
        .call::<((), (), ())>()
        .await?;

    for wallet in wallets.iter() {
        let balance = wallet.get_asset_balance(&mint_asset_id).await?;
        assert_eq!(balance, 3 * amount as u128);
    }

    Ok(())
}

#[tokio::test]
async fn test_contract_instance_get_balances() -> Result<()> {
    let mut rng = thread_rng();
    let signer = PrivateKeySigner::random(&mut rng);
    let (coins, asset_ids) = setup_multiple_assets_coins(signer.address(), 2, 4, 8);

    let random_asset_id = asset_ids[1];
    let provider = setup_test_provider(coins.clone(), vec![], None, None).await?;
    let wallet = Wallet::new(signer, provider.clone());

    setup_program_test!(
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );
    let contract_id = contract_instance.contract_id();

    // Check the current balance of the contract with id 'contract_id'
    let contract_balances = contract_instance.get_balances().await?;
    assert!(contract_balances.is_empty());

    // Transfer an amount to the contract
    let amount = 8;
    wallet
        .force_transfer_to_contract(contract_id, amount, random_asset_id, TxPolicies::default())
        .await?;

    // Check that the contract now has 1 coin
    let contract_balances = contract_instance.get_balances().await?;
    assert_eq!(contract_balances.len(), 1);

    let random_asset_balance = contract_balances.get(&random_asset_id).unwrap();
    assert_eq!(*random_asset_balance, amount);

    Ok(())
}

#[tokio::test]
async fn contract_call_futures_implement_send() -> Result<()> {
    use std::future::Future;

    fn tokio_spawn_imitation<T>(_: T)
    where
        T: Future + Send + 'static,
    {
    }

    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    tokio_spawn_imitation(async move {
        contract_instance
            .methods()
            .initialize_counter(42)
            .call()
            .await
            .unwrap();
    });
    Ok(())
}

#[tokio::test]
async fn test_contract_set_estimation() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "LibContract",
                project = "e2e/sway/contracts/lib_contract"
            ),
            Contract(
                name = "LibContractCaller",
                project = "e2e/sway/contracts/lib_contract_caller"
            ),
        ),
        Deploy(
            name = "lib_contract_instance",
            contract = "LibContract",
            wallet = "wallet",
            random_salt = false,
        ),
        Deploy(
            name = "contract_caller_instance",
            contract = "LibContractCaller",
            wallet = "wallet",
            random_salt = false,
        ),
    );
    let lib_contract_id = lib_contract_instance.contract_id();

    let res = lib_contract_instance.methods().increment(42).call().await?;
    assert_eq!(43, res.value);

    {
        // Should fail due to missing external contracts
        let res = contract_caller_instance
            .methods()
            .increment_from_contract(lib_contract_id, 42)
            .call()
            .await;

        assert!(matches!(
            res,
            Err(Error::Transaction(Reason::Failure { .. }))
        ));
    }

    let res = contract_caller_instance
        .methods()
        .increment_from_contract(lib_contract_id, 42)
        .determine_missing_contracts()
        .await?
        .call()
        .await?;

    assert_eq!(43, res.value);
    Ok(())
}

#[tokio::test]
async fn test_output_variable_contract_id_estimation_multicall() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "LibContract",
                project = "e2e/sway/contracts/lib_contract"
            ),
            Contract(
                name = "LibContractCaller",
                project = "e2e/sway/contracts/lib_contract_caller"
            ),
            Contract(
                name = "TestContract",
                project = "e2e/sway/contracts/contract_test"
            ),
        ),
        Deploy(
            name = "lib_contract_instance",
            contract = "LibContract",
            wallet = "wallet",
            random_salt = false,
        ),
        Deploy(
            name = "contract_caller_instance",
            contract = "LibContractCaller",
            wallet = "wallet",
            random_salt = false,
        ),
        Deploy(
            name = "contract_test_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let lib_contract_id = lib_contract_instance.contract_id();

    let contract_methods = contract_caller_instance.methods();

    let mut multi_call_handler =
        CallHandler::new_multi_call(wallet.clone()).with_tx_policies(Default::default());

    for _ in 0..3 {
        let call_handler = contract_methods.increment_from_contract(lib_contract_id, 42);
        multi_call_handler = multi_call_handler.add_call(call_handler);
    }

    // add call that does not need ContractId
    let contract_methods = contract_test_instance.methods();
    let call_handler = contract_methods.get(5, 6);

    multi_call_handler = multi_call_handler.add_call(call_handler);

    let call_response = multi_call_handler
        .determine_missing_contracts()
        .await?
        .call::<(u64, u64, u64, u64)>()
        .await?;

    assert_eq!(call_response.value, (43, 43, 43, 11));

    Ok(())
}

#[tokio::test]
async fn test_contract_call_with_non_default_max_input() -> Result<()> {
    use fuels::{
        tx::{ConsensusParameters, TxParameters},
        types::coin::Coin,
    };

    let mut consensus_parameters = ConsensusParameters::default();
    let tx_params = TxParameters::default()
        .with_max_inputs(123)
        .with_max_size(1_000_000);
    consensus_parameters.set_tx_params(tx_params);
    let contract_params = ContractParameters::default().with_contract_max_size(1_000_000);
    consensus_parameters.set_contract_params(contract_params);

    let mut rng = thread_rng();
    let signer = PrivateKeySigner::random(&mut rng);

    let coins: Vec<Coin> = setup_single_asset_coins(
        signer.address(),
        Default::default(),
        DEFAULT_NUM_COINS,
        DEFAULT_COIN_AMOUNT,
    );
    let chain_config = ChainConfig {
        consensus_parameters: consensus_parameters.clone(),
        ..ChainConfig::default()
    };

    let provider = setup_test_provider(coins, vec![], None, Some(chain_config)).await?;
    let wallet = Wallet::new(signer, provider.clone());
    assert_eq!(consensus_parameters, provider.consensus_parameters().await?);

    setup_program_test!(
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let response = contract_instance.methods().get(5, 6).call().await?;

    assert_eq!(response.value, 11);

    Ok(())
}

#[tokio::test]
async fn test_add_custom_assets() -> Result<()> {
    let initial_amount = 100_000;
    let asset_base = AssetConfig {
        id: AssetId::zeroed(),
        num_coins: 1,
        coin_amount: initial_amount,
    };

    let asset_id_1 = AssetId::from([3u8; 32]);
    let asset_1 = AssetConfig {
        id: asset_id_1,
        num_coins: 1,
        coin_amount: initial_amount,
    };

    let asset_id_2 = AssetId::from([1u8; 32]);
    let asset_2 = AssetConfig {
        id: asset_id_2,
        num_coins: 1,
        coin_amount: initial_amount,
    };

    let assets = vec![asset_base, asset_1, asset_2];

    let num_wallets = 2;
    let wallet_config = WalletsConfig::new_multiple_assets(num_wallets, assets);
    let mut wallets = launch_custom_provider_and_get_wallets(wallet_config, None, None).await?;
    let wallet_1 = wallets.pop().unwrap();
    let wallet_2 = wallets.pop().unwrap();

    setup_program_test!(
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "MyContract",
            wallet = "wallet_1",
            random_salt = false,
        ),
    );

    let amount_1 = 5000;
    let amount_2 = 3000;
    let response = contract_instance
        .methods()
        .get(5, 6)
        .add_custom_asset(asset_id_1, amount_1, Some(wallet_2.address()))
        .add_custom_asset(asset_id_2, amount_2, Some(wallet_2.address()))
        .call()
        .await?;

    assert_eq!(response.value, 11);

    let balance_asset_1 = wallet_1.get_asset_balance(&asset_id_1).await?;
    let balance_asset_2 = wallet_1.get_asset_balance(&asset_id_2).await?;
    assert_eq!(balance_asset_1, (initial_amount - amount_1) as u128);
    assert_eq!(balance_asset_2, (initial_amount - amount_2) as u128);

    let balance_asset_1 = wallet_2.get_asset_balance(&asset_id_1).await?;
    let balance_asset_2 = wallet_2.get_asset_balance(&asset_id_2).await?;
    assert_eq!(balance_asset_1, (initial_amount + amount_1) as u128);
    assert_eq!(balance_asset_2, (initial_amount + amount_2) as u128);

    Ok(())
}

#[tokio::test]
async fn contract_load_error_messages() {
    {
        let binary_path = "sway/contracts/contract_test/out/release/no_file_on_path.bin";
        let expected_error = format!("io: file \"{binary_path}\" does not exist");

        let error = Contract::load_from(binary_path, LoadConfiguration::default())
            .expect_err("should have failed");

        assert_eq!(error.to_string(), expected_error);
    }
    {
        let binary_path = "sway/contracts/contract_test/out/release/contract_test-abi.json";
        let expected_error = format!("expected \"{binary_path}\" to have '.bin' extension");

        let error = Contract::load_from(binary_path, LoadConfiguration::default())
            .expect_err("should have failed");

        assert_eq!(error.to_string(), expected_error);
    }
}

#[tokio::test]
async fn test_payable_annotation() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/payable_annotation"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let contract_methods = contract_instance.methods();

    let response = contract_methods
        .payable()
        .call_params(
            CallParameters::default()
                .with_amount(100)
                .with_gas_forwarded(20_000),
        )?
        .call()
        .await?;

    assert_eq!(response.value, 42);

    // ANCHOR: non_payable_params
    let err = contract_methods
        .non_payable()
        .call_params(CallParameters::default().with_amount(100))
        .expect_err("should return error");

    assert!(matches!(err, Error::Other(s) if s.contains("assets forwarded to non-payable method")));
    // ANCHOR_END: non_payable_params

    let response = contract_methods
        .non_payable()
        .call_params(CallParameters::default().with_gas_forwarded(20_000))?
        .call()
        .await?;

    assert_eq!(response.value, 42);

    Ok(())
}

#[tokio::test]
async fn multi_call_from_calls_with_different_account_types() -> Result<()> {
    use fuels::prelude::*;

    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/contract_test/out/release/contract_test-abi.json"
    ));

    let wallet = launch_provider_and_get_wallet().await?;
    let predicate = Predicate::from_code(vec![]);

    let contract_methods_wallet = MyContract::new(ContractId::default(), wallet.clone()).methods();
    let contract_methods_predicate = MyContract::new(ContractId::default(), predicate).methods();

    let call_handler_1 = contract_methods_wallet.initialize_counter(42);
    let call_handler_2 = contract_methods_predicate.get_array([42; 2]);

    let _multi_call_handler = CallHandler::new_multi_call(wallet)
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    Ok(())
}

#[tokio::test]
async fn low_level_call() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "MyCallerContract",
                project = "e2e/sway/contracts/low_level_caller"
            ),
            Contract(
                name = "MyTargetContract",
                project = "e2e/sway/contracts/contract_test"
            ),
        ),
        Deploy(
            name = "caller_contract_instance",
            contract = "MyCallerContract",
            wallet = "wallet",
            random_salt = false,
        ),
        Deploy(
            name = "target_contract_instance",
            contract = "MyTargetContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );
    {
        let function_selector = encode_fn_selector("initialize_counter");
        let call_data = calldata!(42u64)?;
        caller_contract_instance
            .methods()
            .call_low_level_call(
                target_contract_instance.id(),
                Bytes(function_selector),
                Bytes(call_data),
            )
            .determine_missing_contracts()
            .await?
            .call()
            .await?;

        let response = target_contract_instance
            .methods()
            .read_counter()
            .call()
            .await?;
        assert_eq!(response.value, 42);
    }
    {
        let function_selector = encode_fn_selector("set_value_multiple_complex");
        let call_data = calldata!(
            MyStruct {
                a: true,
                b: [1, 2, 3],
            },
            SizedAsciiString::<4>::try_from("fuel")?
        )?;

        caller_contract_instance
            .methods()
            .call_low_level_call(
                target_contract_instance.id(),
                Bytes(function_selector),
                Bytes(call_data),
            )
            .determine_missing_contracts()
            .await?
            .call()
            .await?;

        let result_uint = target_contract_instance
            .methods()
            .read_counter()
            .call()
            .await?
            .value;

        let result_bool = target_contract_instance
            .methods()
            .get_bool_value()
            .call()
            .await?
            .value;

        let result_str = target_contract_instance
            .methods()
            .get_str_value()
            .call()
            .await?
            .value;

        assert_eq!(result_uint, 2);
        assert!(result_bool);
        assert_eq!(result_str, "fuel");
    }

    Ok(())
}

#[cfg(any(not(feature = "fuel-core-lib"), feature = "rocksdb"))]
#[test]
fn db_rocksdb() {
    use std::fs;

    use fuels::{
        accounts::wallet::Wallet,
        client::{PageDirection, PaginationRequest},
        prelude::{DEFAULT_COIN_AMOUNT, DbType, Error, ViewOnlyAccount, setup_test_provider},
    };

    let temp_dir = tempfile::tempdir().expect("failed to make tempdir");
    let temp_database_path = temp_dir.path().join("db");

    tokio::runtime::Runtime::new()
        .expect("tokio runtime failed")
        .block_on(async {
            let _ = temp_dir;
            let signer = PrivateKeySigner::new(
                "0x4433d156e8c53bf5b50af07aa95a29436f29a94e0ccc5d58df8e57bdc8583c32"
                    .parse()
                    .unwrap(),
            );

            const NUMBER_OF_ASSETS: u64 = 2;
            let node_config = NodeConfig {
                database_type: DbType::RocksDb(Some(temp_database_path.clone())),
                ..NodeConfig::default()
            };

            let chain_config = ChainConfig {
                consensus_parameters: Default::default(),
                ..ChainConfig::local_testnet()
            };

            let (coins, _) = setup_multiple_assets_coins(
                signer.address(),
                NUMBER_OF_ASSETS,
                DEFAULT_NUM_COINS,
                DEFAULT_COIN_AMOUNT,
            );

            let provider =
                setup_test_provider(coins.clone(), vec![], Some(node_config), Some(chain_config))
                    .await?;

            provider.produce_blocks(2, None).await?;

            Ok::<(), Error>(())
        })
        .unwrap();

    // The runtime needs to be terminated because the node can currently only be killed when the runtime itself shuts down.

    tokio::runtime::Runtime::new()
        .expect("tokio runtime failed")
        .block_on(async {
            let node_config = NodeConfig {
                database_type: DbType::RocksDb(Some(temp_database_path.clone())),
                ..NodeConfig::default()
            };

            let provider = setup_test_provider(vec![], vec![], Some(node_config), None).await?;
            // the same wallet that was used when rocksdb was built. When we connect it to the provider, we expect it to have the same amount of assets
            let signer = PrivateKeySigner::new(
                "0x4433d156e8c53bf5b50af07aa95a29436f29a94e0ccc5d58df8e57bdc8583c32"
                    .parse()
                    .unwrap(),
            );
            let wallet = Wallet::new(signer, provider.clone());

            let blocks = provider
                .get_blocks(PaginationRequest {
                    cursor: None,
                    results: 10,
                    direction: PageDirection::Forward,
                })
                .await?
                .results;

            assert_eq!(blocks.len(), 3);
            assert_eq!(
                *wallet.get_balances().await?.iter().next().unwrap().1,
                DEFAULT_COIN_AMOUNT as u128
            );
            assert_eq!(
                *wallet.get_balances().await?.iter().next().unwrap().1,
                DEFAULT_COIN_AMOUNT as u128
            );
            assert_eq!(wallet.get_balances().await?.len(), 2);

            fs::remove_dir_all(
                temp_database_path
                    .parent()
                    .expect("db parent folder does not exist"),
            )?;

            Ok::<(), Error>(())
        })
        .unwrap();
}

#[tokio::test]
async fn can_configure_decoding_of_contract_return() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/needs_custom_decoder"
        ),),
        Deploy(
            contract = "MyContract",
            name = "contract_instance",
            wallet = "wallet",
            random_salt = false,
        )
    );

    let methods = contract_instance.methods();
    {
        // Single call: Will not work if max_tokens not big enough
        methods.i_return_a_1k_el_array().with_decoder_config(DecoderConfig{max_tokens: 100, ..Default::default()}).call().await.expect_err(
             "should have failed because there are more tokens than what is supported by default",
         );
    }
    {
        // Single call: Works when limit is bumped
        let result = methods
            .i_return_a_1k_el_array()
            .with_decoder_config(DecoderConfig {
                max_tokens: 1001,
                ..Default::default()
            })
            .call()
            .await?
            .value;

        assert_eq!(result, [0; 1000]);
    }
    {
        // Multi call: Will not work if max_tokens not big enough
        CallHandler::new_multi_call(wallet.clone())
         .add_call(methods.i_return_a_1k_el_array())
         .with_decoder_config(DecoderConfig { max_tokens: 100, ..Default::default() })
         .call::<([u8; 1000],)>().await.expect_err(
             "should have failed because there are more tokens than what is supported by default",
         );
    }
    {
        // Multi call: Works when configured
        CallHandler::new_multi_call(wallet.clone())
            .add_call(methods.i_return_a_1k_el_array())
            .with_decoder_config(DecoderConfig {
                max_tokens: 1001,
                ..Default::default()
            })
            .call::<([u8; 1000],)>()
            .await
            .unwrap();
    }

    Ok(())
}

#[tokio::test]
async fn test_contract_submit_and_response() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let contract_methods = contract_instance.methods();

    let submitted_tx = contract_methods.get(1, 2).submit().await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let value = submitted_tx.response().await?.value;

    assert_eq!(value, 3);

    let contract_methods = contract_instance.methods();
    let call_handler_1 = contract_methods.get_single(7);
    let call_handler_2 = contract_methods.get_single(42);

    let multi_call_handler = CallHandler::new_multi_call(wallet.clone())
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let handle = multi_call_handler.submit().await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let (val_1, val_2): (u64, u64) = handle.response().await?.value;

    assert_eq!(val_1, 7);
    assert_eq!(val_2, 42);

    Ok(())
}

#[tokio::test]
async fn test_heap_type_multicall() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "TestContract",
                project = "e2e/sway/contracts/contract_test"
            ),
            Contract(
                name = "VectorOutputContract",
                project = "e2e/sway/types/contracts/vector_output"
            )
        ),
        Deploy(
            name = "contract_instance",
            contract = "VectorOutputContract",
            wallet = "wallet",
            random_salt = false,
        ),
        Deploy(
            name = "contract_instance_2",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    {
        let call_handler_1 = contract_instance.methods().u8_in_vec(5);
        let call_handler_2 = contract_instance_2.methods().get_single(7);
        let call_handler_3 = contract_instance.methods().u8_in_vec(3);

        let multi_call_handler = CallHandler::new_multi_call(wallet.clone())
            .add_call(call_handler_1)
            .add_call(call_handler_2)
            .add_call(call_handler_3);

        let (val_1, val_2, val_3): (Vec<u8>, u64, Vec<u8>) = multi_call_handler.call().await?.value;

        assert_eq!(val_1, vec![0, 1, 2, 3, 4]);
        assert_eq!(val_2, 7);
        assert_eq!(val_3, vec![0, 1, 2]);
    }

    Ok(())
}

#[tokio::test]
async fn heap_types_correctly_offset_in_create_transactions_w_storage_slots() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Predicate(
            name = "MyPredicate",
            project = "e2e/sway/types/predicates/predicate_vector"
        ),),
    );

    let provider = wallet.provider().clone();
    let data = MyPredicateEncoder::default().encode_data(18, 24, vec![2, 4, 42])?;
    let predicate = Predicate::load_from(
        "sway/types/predicates/predicate_vector/out/release/predicate_vector.bin",
    )?
    .with_data(data)
    .with_provider(provider);

    wallet
        .transfer(
            predicate.address(),
            10_000,
            AssetId::zeroed(),
            TxPolicies::default(),
        )
        .await?;

    // if the contract is successfully deployed then the predicate was unlocked. This further means
    // the offsets were setup correctly since the predicate uses heap types in its arguments.
    // Storage slots were loaded automatically by default
    Contract::load_from(
        "sway/contracts/storage/out/release/storage.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&predicate, TxPolicies::default())
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_arguments_with_gas_forwarded() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(
            Contract(
                name = "TestContract",
                project = "e2e/sway/contracts/contract_test"
            ),
            Contract(
                name = "VectorOutputContract",
                project = "e2e/sway/types/contracts/vectors"
            )
        ),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
        Deploy(
            name = "contract_instance_2",
            contract = "VectorOutputContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let x = 128;
    let vec_input = vec![0, 1, 2];
    {
        let response = contract_instance
            .methods()
            .get_single(x)
            .call_params(CallParameters::default().with_gas_forwarded(4096))?
            .call()
            .await?;

        assert_eq!(response.value, x);
    }
    {
        contract_instance_2
            .methods()
            .u32_vec(vec_input.clone())
            .call_params(CallParameters::default().with_gas_forwarded(4096))?
            .call()
            .await?;
    }
    {
        let call_handler_1 = contract_instance.methods().get_single(x);
        let call_handler_2 = contract_instance_2.methods().u32_vec(vec_input);

        let multi_call_handler = CallHandler::new_multi_call(wallet.clone())
            .add_call(call_handler_1)
            .add_call(call_handler_2);

        let (value, _): (u64, ()) = multi_call_handler.call().await?.value;

        assert_eq!(value, x);
    }

    Ok(())
}

#[tokio::test]
async fn contract_custom_call_no_signatures_strategy() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "TestContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );
    let provider = wallet.provider();

    let counter = 42;
    let call_handler = contract_instance.methods().initialize_counter(counter);

    let mut tb = call_handler.transaction_builder().await?;

    let base_asset_id = *provider.consensus_parameters().await?.base_asset_id();

    let amount = 10;
    let consensus_parameters = provider.consensus_parameters().await?;
    let new_base_inputs = wallet
        .get_asset_inputs_for_amount(base_asset_id, amount, None)
        .await?;
    tb.inputs_mut().extend(new_base_inputs);
    tb.outputs_mut()
        .push(Output::change(wallet.address(), 0, base_asset_id));

    // ANCHOR: tb_no_signatures_strategy
    let mut tx = tb
        .with_build_strategy(ScriptBuildStrategy::NoSignatures)
        .build(provider)
        .await?;
    // ANCHOR: tx_sign_with
    tx.sign_with(wallet.signer(), consensus_parameters.chain_id())
        .await?;
    // ANCHOR_END: tx_sign_with
    // ANCHOR_END: tb_no_signatures_strategy

    let tx_id = provider.send_transaction(tx).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let tx_status = provider.tx_status(&tx_id).await?;

    let response = call_handler.get_response(tx_status)?;

    assert_eq!(counter, response.value);

    Ok(())
}

#[tokio::test]
async fn contract_encoder_config_is_applied() -> Result<()> {
    setup_program_test!(
        Abigen(Contract(
            name = "TestContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Wallets("wallet")
    );
    let contract_id = Contract::load_from(
        "sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&wallet, TxPolicies::default())
    .await?
    .contract_id;

    let instance = TestContract::new(contract_id, wallet.clone());

    {
        let _encoding_ok = instance
            .methods()
            .get(0, 1)
            .call()
            .await
            .expect("should not fail as it uses the default encoder config");
    }
    {
        let encoder_config = EncoderConfig {
            max_tokens: 1,
            ..Default::default()
        };
        let instance_with_encoder_config = instance.with_encoder_config(encoder_config);

        // uses 2 tokens when 1 is the limit
        let encoding_error = instance_with_encoder_config
            .methods()
            .get(0, 1)
            .call()
            .await
            .expect_err("should error");

        assert!(encoding_error.to_string().contains(
            "cannot encode contract call arguments: codec: token limit `1` reached while encoding."
        ));

        let encoding_error = instance_with_encoder_config
            .methods()
            .get(0, 1)
            .simulate(Execution::realistic())
            .await
            .expect_err("should error");

        assert!(encoding_error.to_string().contains(
            "cannot encode contract call arguments: codec: token limit `1` reached while encoding."
        ));
    }

    Ok(())
}

#[tokio::test]
async fn test_reentrant_calls() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "LibContractCaller",
            project = "e2e/sway/contracts/lib_contract_caller"
        ),),
        Deploy(
            name = "contract_caller_instance",
            contract = "LibContractCaller",
            wallet = "wallet",
            random_salt = false,
        ),
    );

    let contract_id = contract_caller_instance.contract_id();
    let response = contract_caller_instance
        .methods()
        .re_entrant(contract_id, true)
        .call()
        .await?;

    assert_eq!(42, response.value);

    Ok(())
}

#[tokio::test]
async fn msg_sender_gas_estimation_issue() {
    // Gas estimation requires an input of the base asset. If absent, a fake input is
    // added. However, if a non-base coin is present and the fake input introduces a
    // second owner, it causes the `msg_sender` sway fn to fail. This leads
    // to a premature failure in gas estimation, risking transaction failure due to
    // a low gas limit.
    let mut rng = thread_rng();
    let signer = PrivateKeySigner::random(&mut rng);
    let (coins, ids) =
        setup_multiple_assets_coins(signer.address(), 2, DEFAULT_NUM_COINS, DEFAULT_COIN_AMOUNT);
    let provider = setup_test_provider(coins, vec![], None, None)
        .await
        .unwrap();
    let wallet = Wallet::new(signer, provider.clone());

    setup_program_test!(
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/msg_methods"
        )),
        Deploy(
            contract = "MyContract",
            name = "contract_instance",
            wallet = "wallet",
            random_salt = false,
        )
    );

    let asset_id = ids[0];

    // The fake coin won't be added if we add a base asset, so let's not do that
    assert!(
        asset_id
            != *provider
                .consensus_parameters()
                .await
                .unwrap()
                .base_asset_id()
    );
    let call_params = CallParameters::default()
        .with_amount(100)
        .with_asset_id(asset_id);

    contract_instance
        .methods()
        .message_sender()
        .call_params(call_params)
        .unwrap()
        .call()
        .await
        .unwrap();
}

#[tokio::test]
async fn variable_output_estimation_is_optimized() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/var_outputs"
        )),
        Deploy(
            contract = "MyContract",
            name = "contract_instance",
            wallet = "wallet",
            random_salt = false,
        )
    );

    let contract_methods = contract_instance.methods();

    let coins = 252;
    let recipient = Identity::Address(wallet.address());
    let start = Instant::now();
    let _ = contract_methods
        .mint(coins, recipient)
        .with_variable_output_policy(VariableOutputPolicy::EstimateMinimum)
        .call()
        .await?;

    // debug builds are slower (20x for `fuel-core-lib`, 4x for a release-fuel-core-binary)
    // we won't validate in that case so we don't have to maintain two expectations
    if !cfg!(debug_assertions) {
        let elapsed = start.elapsed().as_secs();
        let limit = 2;
        if elapsed > limit {
            panic!("Estimation took too long ({elapsed}). Limit is {limit}");
        }
    }

    Ok(())
}

fn config_for_high_price_node() -> (WalletsConfig, NodeConfig, ChainConfig) {
    let wallet_config = WalletsConfig::new(None, None, None);
    let fee_parameters = FeeParameters::V1(FeeParametersV1 {
        gas_price_factor: 92000,
        gas_per_byte: 63,
    });
    let consensus_parameters = ConsensusParameters::V1(ConsensusParametersV1 {
        fee_params: fee_parameters,
        ..Default::default()
    });

    let node_config = NodeConfig {
        starting_gas_price: 1100,
        ..NodeConfig::default()
    };

    let chain_config = ChainConfig {
        consensus_parameters,
        ..ChainConfig::default()
    };

    (wallet_config, node_config, chain_config)
}

async fn setup_node_with_high_price() -> Result<Vec<Wallet>> {
    let (wallet_config, node_config, chain_config) = config_for_high_price_node();

    let wallets = launch_custom_provider_and_get_wallets(
        wallet_config,
        Some(node_config),
        Some(chain_config),
    )
    .await?;

    Ok(wallets)
}

#[cfg(any(not(feature = "fuel-core-lib"), feature = "rocksdb"))]
async fn setup_node_with_high_price_historical_execution()
-> Result<(Vec<Wallet>, tempfile::TempDir)> {
    let (wallet_config, mut node_config, chain_config) = config_for_high_price_node();

    let temp_dir = tempfile::tempdir().expect("failed to make tempdir");
    let temp_database_path = temp_dir.path().join("db");

    node_config.database_type = DbType::RocksDb(Some(temp_database_path));
    node_config.historical_execution = true;

    let wallets = launch_custom_provider_and_get_wallets(
        wallet_config,
        Some(node_config),
        Some(chain_config),
    )
    .await?;

    Ok((wallets, temp_dir))
}

#[tokio::test]
async fn simulations_can_be_made_without_coins() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/contract_test/out/release/contract_test-abi.json"
    ));

    let wallet = setup_node_with_high_price().await?.pop().unwrap();

    let contract_id = Contract::load_from(
        "sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&wallet, TxPolicies::default())
    .await?
    .contract_id;

    let provider = wallet.provider().clone();
    let no_funds_wallet = Wallet::random(&mut thread_rng(), provider);

    let response = MyContract::new(contract_id, no_funds_wallet)
        .methods()
        .get(5, 6)
        .simulate(Execution::state_read_only())
        .await?;

    assert_eq!(response.value, 11);

    Ok(())
}

#[tokio::test]
#[cfg(any(not(feature = "fuel-core-lib"), feature = "rocksdb"))]
async fn simulations_can_be_made_at_specific_block_height() -> Result<()> {
    let (mut wallets, _temp_dir) = setup_node_with_high_price_historical_execution().await?;
    let wallet = wallets.pop().unwrap();

    setup_program_test!(
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            contract = "MyContract",
            wallet = "wallet",
            random_salt = false,
        ),
    );
    let contract_id = contract_instance.contract_id();

    let provider = wallet.provider();
    let contract_methods = contract_instance.methods();

    contract_methods.initialize_counter(42).call().await?;
    provider.produce_blocks(5, None).await?;
    let block_height = provider.latest_block_height().await?;

    provider.produce_blocks(5, None).await?;
    contract_methods.increment_counter(24).call().await?;

    let no_funds_wallet = Wallet::random(&mut thread_rng(), provider.clone());
    let no_funds_methods = MyContract::new(contract_id, no_funds_wallet).methods();
    {
        let response = no_funds_methods
            .read_counter()
            .simulate(Execution::state_read_only())
            .await?;

        assert_eq!(response.value, 66);
    }
    {
        let response = no_funds_methods
            .read_counter()
            .simulate(Execution::state_read_only().at_height(block_height))
            .await?;

        assert_eq!(response.value, 42);
    }

    Ok(())
}

#[tokio::test]
async fn simulations_can_be_made_without_coins_multicall() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/contract_test/out/release/contract_test-abi.json"
    ));

    let wallet = setup_node_with_high_price().await?.pop().unwrap();

    let contract_id = Contract::load_from(
        "sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(&wallet, TxPolicies::default())
    .await?
    .contract_id;

    let provider = wallet.provider().clone();

    let no_funds_wallet = Wallet::random(&mut thread_rng(), provider);
    let contract_instance = MyContract::new(contract_id, no_funds_wallet.clone());

    let contract_methods = contract_instance.methods();

    let call_handler_1 = contract_methods.get(1, 2);
    let call_handler_2 = contract_methods.get(3, 4);

    let mut multi_call_handler = CallHandler::new_multi_call(no_funds_wallet)
        .add_call(call_handler_1)
        .add_call(call_handler_2);

    let value: (u64, u64) = multi_call_handler
        .simulate(Execution::state_read_only())
        .await?
        .value;

    assert_eq!(value, (3, 7));

    Ok(())
}

#[tokio::test]
async fn contract_call_with_non_zero_base_asset_id_and_tip() -> Result<()> {
    use fuels::{prelude::*, tx::ConsensusParameters};

    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/contract_test/out/release/contract_test-abi.json"
    ));

    let asset_id = AssetId::new([1; 32]);

    let mut consensus_parameters = ConsensusParameters::default();
    consensus_parameters.set_base_asset_id(asset_id);

    let config = ChainConfig {
        consensus_parameters,
        ..Default::default()
    };

    let asset_base = AssetConfig {
        id: asset_id,
        num_coins: 1,
        coin_amount: 10_000,
    };

    let wallet_config = WalletsConfig::new_multiple_assets(1, vec![asset_base]);
    let wallets = launch_custom_provider_and_get_wallets(wallet_config, None, Some(config)).await?;
    let wallet = wallets.first().expect("has wallet");

    let contract_id = Contract::load_from(
        "sway/contracts/contract_test/out/release/contract_test.bin",
        LoadConfiguration::default(),
    )?
    .deploy_if_not_exists(wallet, TxPolicies::default())
    .await?
    .contract_id;

    let contract_instance = MyContract::new(contract_id, wallet.clone());

    let response = contract_instance
        .methods()
        .initialize_counter(42)
        .with_tx_policies(TxPolicies::default().with_tip(10))
        .call()
        .await?;

    assert_eq!(42, response.value);

    Ok(())
}

#[tokio::test]
async fn max_fee_estimation_respects_tolerance() -> Result<()> {
    use fuels::prelude::*;

    let mut rng = rand::thread_rng();
    let call_signer = PrivateKeySigner::random(&mut rng);

    let call_coins = setup_single_asset_coins(call_signer.address(), AssetId::BASE, 1000, 1);

    let deploy_signer = PrivateKeySigner::random(&mut rng);
    let deploy_coins =
        setup_single_asset_coins(deploy_signer.address(), AssetId::BASE, 1, 1_000_000);

    let provider =
        setup_test_provider([call_coins, deploy_coins].concat(), vec![], None, None).await?;

    let call_wallet = Wallet::new(call_signer, provider.clone());
    let deploy_wallet = Wallet::new(deploy_signer, provider.clone());

    setup_program_test!(
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/contract_test"
        )),
        Deploy(
            name = "contract_instance",
            wallet = "deploy_wallet",
            contract = "MyContract",
            random_salt = false,
        )
    );
    let contract_instance = contract_instance.with_account(call_wallet.clone());

    let max_fee_from_tx = |tolerance: f32| {
        let contract_instance = contract_instance.clone();
        let provider = provider.clone();
        async move {
            let builder = contract_instance
                .methods()
                .initialize_counter(42)
                .transaction_builder()
                .await
                .unwrap();

            assert_eq!(
                builder.max_fee_estimation_tolerance, DEFAULT_MAX_FEE_ESTIMATION_TOLERANCE,
                "Expected pre-set tolerance"
            );

            builder
                .with_max_fee_estimation_tolerance(tolerance)
                .build(&provider)
                .await
                .unwrap()
                .max_fee()
                .unwrap()
        }
    };

    let max_fee_from_builder = |tolerance: f32| {
        let contract_instance = contract_instance.clone();
        let provider = provider.clone();
        async move {
            contract_instance
                .methods()
                .initialize_counter(42)
                .transaction_builder()
                .await
                .unwrap()
                .with_max_fee_estimation_tolerance(tolerance)
                .estimate_max_fee(&provider)
                .await
                .unwrap()
        }
    };

    let base_amount_in_inputs = |tolerance: f32| {
        let contract_instance = contract_instance.clone();
        let call_wallet = &call_wallet;
        async move {
            let mut tb = contract_instance
                .methods()
                .initialize_counter(42)
                .transaction_builder()
                .await
                .unwrap()
                .with_max_fee_estimation_tolerance(tolerance);

            call_wallet.adjust_for_fee(&mut tb, 0).await.unwrap();
            tb.inputs
                .iter()
                .filter_map(|input: &Input| match input {
                    Input::ResourceSigned { resource }
                        if resource.coin_asset_id().unwrap() == AssetId::BASE =>
                    {
                        Some(resource.amount())
                    }
                    _ => None,
                })
                .sum::<u64>()
        }
    };

    let no_increase_max_fee = max_fee_from_tx(0.0).await;
    let increased_max_fee = max_fee_from_tx(2.00).await;

    assert_eq!(
        increased_max_fee as f64 / no_increase_max_fee as f64,
        1.00 + 2.00
    );

    let no_increase_max_fee = max_fee_from_builder(0.0).await;
    let increased_max_fee = max_fee_from_builder(2.00).await;
    assert_eq!(
        increased_max_fee as f64 / no_increase_max_fee as f64,
        1.00 + 2.00
    );

    let normal_base_asset = base_amount_in_inputs(0.0).await;
    let more_base_asset_due_to_bigger_tolerance = base_amount_in_inputs(5.00).await;
    assert!(more_base_asset_due_to_bigger_tolerance > normal_base_asset);

    Ok(())
}

#[tokio::test]
async fn blob_contract_deployment() -> Result<()> {
    abigen!(Contract(
        name = "MyContract",
        abi = "e2e/sway/contracts/huge_contract/out/release/huge_contract-abi.json"
    ));

    let contract_binary = "sway/contracts/huge_contract/out/release/huge_contract.bin";
    let contract_size = std::fs::metadata(contract_binary)
        .expect("contract file not found")
        .len();

    assert!(
        contract_size > 150_000,
        "the testnet size limit was around 100kB, we want a contract bigger than that to reflect prod (current: {contract_size}B)"
    );

    let wallets =
        launch_custom_provider_and_get_wallets(WalletsConfig::new(Some(2), None, None), None, None)
            .await?;

    let provider = wallets[0].provider().clone();

    let consensus_parameters = provider.consensus_parameters().await?;

    let contract_max_size = consensus_parameters.contract_params().contract_max_size();
    assert!(
        contract_size > contract_max_size,
        "this test should ideally be run with a contract bigger than the max contract size ({contract_max_size}B) so that we know deployment couldn't have happened without blobs"
    );

    let contract = Contract::load_from(contract_binary, LoadConfiguration::default())?;

    let contract_id = contract
        .convert_to_loader(100_000)?
        .deploy_if_not_exists(&wallets[0], TxPolicies::default())
        .await?
        .contract_id;

    let contract_instance = MyContract::new(contract_id, wallets[0].clone());

    let response = contract_instance.methods().something().call().await?.value;

    assert_eq!(response, 1001);

    Ok(())
}

#[tokio::test]
async fn regular_contract_can_be_deployed() -> Result<()> {
    // given
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/contract_test"
        )),
    );

    let contract_binary = "sway/contracts/contract_test/out/release/contract_test.bin";

    // when
    let contract_id = Contract::load_from(contract_binary, LoadConfiguration::default())?
        .deploy_if_not_exists(&wallet, TxPolicies::default())
        .await?
        .contract_id;

    // then
    let contract_instance = MyContract::new(contract_id, wallet);

    let response = contract_instance
        .methods()
        .read_counter()
        .call()
        .await?
        .value;

    assert_eq!(response, 0);

    Ok(())
}

#[tokio::test]
async fn unuploaded_loader_can_be_deployed_directly() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/huge_contract"
        )),
    );

    let contract_binary = "sway/contracts/huge_contract/out/release/huge_contract.bin";

    let contract_id = Contract::load_from(contract_binary, LoadConfiguration::default())?
        .convert_to_loader(1024)?
        .deploy_if_not_exists(&wallet, TxPolicies::default())
        .await?
        .contract_id;

    let contract_instance = MyContract::new(contract_id, wallet);

    let response = contract_instance.methods().something().call().await?.value;

    assert_eq!(response, 1001);

    Ok(())
}

#[tokio::test]
async fn unuploaded_loader_can_upload_blobs_separately_then_deploy() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/huge_contract"
        )),
    );

    let contract_binary = "sway/contracts/huge_contract/out/release/huge_contract.bin";

    let contract = Contract::load_from(contract_binary, LoadConfiguration::default())?
        .convert_to_loader(1024)?
        .upload_blobs(&wallet, TxPolicies::default())
        .await?;

    let blob_ids = contract.blob_ids();

    // if this were an example for the user we'd just call `deploy` on the contract above
    // this way we are testing that the blobs were really deployed above, otherwise the following
    // would fail

    let contract_id = Contract::loader_from_blob_ids(
        blob_ids.to_vec(),
        contract.salt(),
        contract.storage_slots().to_vec(),
    )?
    .deploy_if_not_exists(&wallet, TxPolicies::default())
    .await?
    .contract_id;

    let contract_instance = MyContract::new(contract_id, wallet);
    let response = contract_instance.methods().something().call().await?.value;
    assert_eq!(response, 1001);

    Ok(())
}

#[tokio::test]
async fn loader_blob_already_uploaded_not_an_issue() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/huge_contract"
        )),
    );

    let contract_binary = "sway/contracts/huge_contract/out/release/huge_contract.bin";
    let contract = Contract::load_from(contract_binary, LoadConfiguration::default())?
        .convert_to_loader(1024)?;

    // this will upload blobs
    contract
        .clone()
        .upload_blobs(&wallet, TxPolicies::default())
        .await?;

    // this will try to upload the blobs but skip upon encountering an error
    let contract_id = contract
        .deploy_if_not_exists(&wallet, TxPolicies::default())
        .await?
        .contract_id;

    let contract_instance = MyContract::new(contract_id, wallet);
    let response = contract_instance.methods().something().call().await?.value;
    assert_eq!(response, 1001);

    Ok(())
}

#[tokio::test]
async fn loader_works_via_proxy() -> Result<()> {
    let wallet = launch_provider_and_get_wallet().await?;

    abigen!(
        Contract(
            name = "MyContract",
            abi = "e2e/sway/contracts/huge_contract/out/release/huge_contract-abi.json"
        ),
        Contract(
            name = "MyProxy",
            abi = "e2e/sway/contracts/proxy/out/release/proxy-abi.json"
        )
    );

    let contract_binary = "sway/contracts/huge_contract/out/release/huge_contract.bin";

    let contract = Contract::load_from(contract_binary, LoadConfiguration::default())?;

    let contract_id = contract
        .convert_to_loader(100)?
        .deploy_if_not_exists(&wallet, TxPolicies::default())
        .await?
        .contract_id;

    let contract_binary = "sway/contracts/proxy/out/release/proxy.bin";

    let proxy_id = Contract::load_from(contract_binary, LoadConfiguration::default())?
        .deploy_if_not_exists(&wallet, TxPolicies::default())
        .await?
        .contract_id;

    let proxy = MyProxy::new(proxy_id, wallet.clone());
    proxy
        .methods()
        .set_target_contract(contract_id)
        .call()
        .await?;

    let response = proxy
        .methods()
        .something()
        .with_contract_ids(&[contract_id])
        .call()
        .await?
        .value;

    assert_eq!(response, 1001);

    Ok(())
}

#[tokio::test]
async fn loader_storage_works_via_proxy() -> Result<()> {
    let wallet = launch_provider_and_get_wallet().await?;

    abigen!(
        Contract(
            name = "MyContract",
            abi = "e2e/sway/contracts/huge_contract/out/release/huge_contract-abi.json"
        ),
        Contract(
            name = "MyProxy",
            abi = "e2e/sway/contracts/proxy/out/release/proxy-abi.json"
        )
    );

    let contract_binary = "sway/contracts/huge_contract/out/release/huge_contract.bin";

    let contract = Contract::load_from(contract_binary, LoadConfiguration::default())?;
    let contract_storage_slots = contract.storage_slots().to_vec();

    let contract_id = contract
        .convert_to_loader(100)?
        .deploy_if_not_exists(&wallet, TxPolicies::default())
        .await?
        .contract_id;

    let contract_binary = "sway/contracts/proxy/out/release/proxy.bin";
    let proxy_contract = Contract::load_from(contract_binary, LoadConfiguration::default())?;

    let combined_storage_slots = [&contract_storage_slots, proxy_contract.storage_slots()].concat();

    let proxy_id = proxy_contract
        .with_storage_slots(combined_storage_slots)
        .deploy_if_not_exists(&wallet, TxPolicies::default())
        .await?
        .contract_id;

    let proxy = MyProxy::new(proxy_id, wallet.clone());
    proxy
        .methods()
        .set_target_contract(contract_id)
        .call()
        .await?;

    let response = proxy
        .methods()
        .read_some_u64()
        .with_contract_ids(&[contract_id])
        .call()
        .await?
        .value;

    assert_eq!(response, 42);

    let _res = proxy
        .methods()
        .write_some_u64(36)
        .with_contract_ids(&[contract_id])
        .call()
        .await?;

    let response = proxy
        .methods()
        .read_some_u64()
        .with_contract_ids(&[contract_id])
        .call()
        .await?
        .value;

    assert_eq!(response, 36);

    Ok(())
}

#[tokio::test]
async fn adjust_for_fee_errors() -> Result<()> {
    setup_program_test!(
        Wallets("wallet"),
        Abigen(Contract(
            name = "MyContract",
            project = "e2e/sway/contracts/contract_test"
        )),
    );

    let contract_binary = "sway/contracts/contract_test/out/release/contract_test.bin";

    let err = Contract::load_from(contract_binary, LoadConfiguration::default())?
        .deploy(&wallet, TxPolicies::default().with_tip(10_000_000_000_000))
        .await
        .expect_err("should return error");

    assert!(
        matches!(err, Error::Provider(s) if s.contains("failed to adjust inputs to cover for missing \
                base asset: failed to get base asset \
                (0000000000000000000000000000000000000000000000000000000000000000) inputs with amount:"))
    );

    Ok(())
}

#[tokio::test]
async fn tx_input_output() -> Result<()> {
    let [wallet_1, wallet_2] = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(Some(2), Some(10), Some(1000)),
        None,
        None,
    )
    .await?
    .try_into()
    .unwrap();

    abigen!(Contract(
        name = "TxContract",
        abi = "e2e/sway/contracts/tx_input_output/out/release/tx_input_output-abi.json"
    ));
    let contract_binary = "sway/contracts/tx_input_output/out/release/tx_input_output.bin";

    // Set `wallet_1` as the custom input owner
    let configurables = TxContractConfigurables::default().with_OWNER(wallet_1.address())?;

    let contract = Contract::load_from(
        contract_binary,
        LoadConfiguration::default().with_configurables(configurables),
    )?;

    let contract_id = contract
        .deploy_if_not_exists(&wallet_2, TxPolicies::default())
        .await?
        .contract_id;

    let contract_instance = TxContract::new(contract_id, wallet_2.clone());
    let asset_id = AssetId::zeroed();

    {
        let custom_input = wallet_1
            .get_asset_inputs_for_amount(asset_id, 10, None)
            .await?
            .pop()
            .unwrap();

        // Input at first position is a coin owned by wallet_1
        let _ = contract_instance
            .methods()
            .check_input(0)
            .with_inputs(vec![custom_input])
            .add_signer(wallet_1.signer().clone())
            .call()
            .await?;

        let custom_output = Output::change(wallet_1.address(), 0, asset_id);

        // Output at first position is change to wallet_1
        let _ = contract_instance
            .methods()
            .check_output_is_change(0)
            .with_outputs(vec![custom_output])
            .call()
            .await?;
    }
    {
        // Input at first position is not a coin owned by wallet_1
        let err = contract_instance
            .methods()
            .check_input(0)
            .call()
            .await
            .unwrap_err();

        assert!(err.to_string().contains("input is not a coin"));

        // Output at first position is not change
        let err = contract_instance
            .methods()
            .check_output_is_change(0)
            .call()
            .await
            .unwrap_err();

        assert!(err.to_string().contains("output is not change"));
    }

    Ok(())
}

#[tokio::test]
async fn multicall_tx_input_output() -> Result<()> {
    let [wallet_1, wallet_2, wallet_3] = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(Some(3), Some(10), Some(1000)),
        None,
        None,
    )
    .await?
    .try_into()
    .unwrap();

    abigen!(Contract(
        name = "TxContract",
        abi = "e2e/sway/contracts/tx_input_output/out/release/tx_input_output-abi.json"
    ));
    let contract_binary = "sway/contracts/tx_input_output/out/release/tx_input_output.bin";

    let get_contract_instance = |owner: Address| {
        let wallet_for_fees = wallet_3.clone();

        async move {
            let configurables = TxContractConfigurables::default().with_OWNER(owner)?;

            let contract = Contract::load_from(
                contract_binary,
                LoadConfiguration::default().with_configurables(configurables),
            )?;

            let contract_id = contract
                .deploy_if_not_exists(&wallet_for_fees, TxPolicies::default())
                .await?
                .contract_id;

            fuels::types::errors::Result::<_>::Ok(TxContract::new(contract_id, wallet_for_fees))
        }
    };

    // Set `wallet_1` as owner
    let contract_instance_1 = get_contract_instance(wallet_1.address()).await?;
    // Set `wallet_2` as owner
    let contract_instance_2 = get_contract_instance(wallet_2.address()).await?;
    let asset_id = AssetId::zeroed();

    {
        let custom_input = wallet_1
            .get_asset_inputs_for_amount(asset_id, 10, None)
            .await?
            .pop()
            .unwrap();

        // Input at first position is a coin owned by wallet_1
        let ch1 = contract_instance_1
            .methods()
            .check_input(0)
            .with_inputs(vec![custom_input])
            .add_signer(wallet_1.signer().clone());

        let custom_input = wallet_2
            .get_asset_inputs_for_amount(asset_id, 10, None)
            .await?
            .pop()
            .unwrap();

        // As inputs follow the order off calls added to the multicall,
        // we need to check the second input in this call
        let ch2 = contract_instance_2
            .methods()
            .check_input(1)
            .with_inputs(vec![custom_input])
            .add_signer(wallet_2.signer().clone());

        let multi_call_handler = CallHandler::new_multi_call(wallet_3.clone())
            .add_call(ch1)
            .add_call(ch2);

        let _: ((), ()) = multi_call_handler.call().await?.value;
    }
    {
        let custom_input = wallet_1
            .get_asset_inputs_for_amount(asset_id, 10, None)
            .await?
            .pop()
            .unwrap();

        // Input at first position is a coin owned by wallet_1
        let ch1 = contract_instance_1
            .methods()
            .check_input(0)
            .with_inputs(vec![custom_input])
            .add_signer(wallet_1.signer().clone());

        // This call will read the wrong input and return an error
        let ch2 = contract_instance_2.methods().check_input(0);

        let multi_call_handler = CallHandler::new_multi_call(wallet_3.clone())
            .add_call(ch1)
            .add_call(ch2);

        let err = multi_call_handler.call::<((), ())>().await.unwrap_err();

        assert!(err.to_string().contains("wrong owner"));
    }
    {
        let custom_output = Output::change(wallet_1.address(), 0, asset_id);

        // Output at first position is change to wallet_1
        let ch1 = contract_instance_1
            .methods()
            .check_output_is_change(0)
            .with_outputs(vec![custom_output]);

        // This call will read the wrong output and return an error
        let ch2 = contract_instance_2.methods().check_output_is_change(0);

        let multi_call_handler = CallHandler::new_multi_call(wallet_3.clone())
            .add_call(ch1)
            .add_call(ch2);

        let err = multi_call_handler.call::<((), ())>().await.unwrap_err();

        assert!(err.to_string().contains("wrong change address"));
    }

    Ok(())
}
