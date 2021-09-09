#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::boxed::Box;
use sp_std::vec;
use sp_std::vec::*;

use pallet_contracts_registry::RegistryContract;

use t3rn_primitives::abi::{ContractActionDesc, GatewayABIConfig};
use t3rn_primitives::transfers::BalanceOf;
use t3rn_primitives::transfers::TransferEntry;
use t3rn_primitives::CircuitOutboundMessage;

use frame_support::{traits::Get, weights::Weight};

use t3rn_primitives::Compose;
use t3rn_primitives::{GatewayInboundProtocol, GatewayPointer, GatewayType, GatewayVendor};

use volatile_vm::exec::Stack;
use volatile_vm::exec::StackExtension;
use volatile_vm::gas::GasMeter as VVMGasMeter;
use volatile_vm::storage::RawAliveContractInfo;
use volatile_vm::wasm::PrefabWasmModule;
use volatile_vm::VolatileVM;
use volatile_vm::{CallStamp, DeferredStorageWrite, ExecReturnValue};
pub mod volatile_vm_impl;

use crate::exec_composer::volatile_vm_impl::*;
use crate::AuthorityId;
use crate::Config;

use sp_core::Hasher;
use volatile_vm::exec::FrameArgs;
use volatile_vm::wasm::RunMode;

type ChainId = [u8; 4];

use sp_runtime::create_runtime_str;
use sp_version::RuntimeVersion;

pub const TEST_RUNTIME_VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("test-runtime"),
    impl_name: create_runtime_str!("test-runtime"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 1,
    apis: sp_version::create_apis_vec!([]),
    transaction_version: 1,
};
pub struct ExecComposer {}

impl ExecComposer {
    pub fn post_run_single_contract<T: Config>(
        contract: RegistryContract<T::Hash, T::AccountId, BalanceOf<T>, T::BlockNumber>,
        _escrow_account: T::AccountId,
        _submitter: AuthorityId,
        _requester: T::AccountId,
        target_dest: T::AccountId,
        value: BalanceOf<T>,
        input: Vec<u8>,
        gateway_id: bp_runtime::ChainId,
        _gateway_abi_config: GatewayABIConfig,
        _confirmed_outputs: Vec<u8>,
    ) -> Result<Vec<CircuitOutboundMessage>, &'static str> {
        let _output_mode = StuffedOutputMode::new();

        let (name, code_txt, _gateway_id, exec_type, dest, value, bytes, input_data) = (
            vec![],
            contract.code_txt,
            gateway_id,
            vec![],
            target_dest,
            value,
            contract.bytes,
            input,
        );
        let _compose = Compose {
            name,
            code_txt,
            exec_type,
            dest,
            value,
            bytes,
            input_data,
        };

        // confirmed_outputs =? collected_artifacts
        Ok(vec![])
    }

    pub fn dry_run_single_contract<T: Config>(
        compose: Compose<T::AccountId, BalanceOf<T>>,
    ) -> Result<RegistryContract<T::Hash, T::AccountId, BalanceOf<T>, T::BlockNumber>, &'static str>
    {
        let contract_action_descriptions: Vec<ContractActionDesc<T::Hash, ChainId, T::AccountId>> =
            vec![];

        let mut temp_contract = RegistryContract::from_compose(
            compose.clone(),
            contract_action_descriptions,
            Default::default(),
            None,
            None,
            Some(RawAliveContractInfo {
                trie_id: Default::default(),
                storage_size: Default::default(),
                pair_count: Default::default(),
                code_hash: T::Hashing::hash(&compose.bytes),
                rent_allowance: Default::default(),
                rent_paid: Default::default(),
                deduct_block: Default::default(),
                last_write: Default::default(),
                _reserved: Default::default(),
            }),
            Default::default(),
        );

        Self::preload_bunch_of_contracts::<T>(vec![temp_contract.clone()], Default::default())?;

        Self::run_single_contract::<T, OptimisticOutputMode>(
            &mut temp_contract,
            Default::default(),
            Weight::MAX,
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )?;

        Ok(temp_contract)
    }

    pub fn run_single_contract<T: Config, OM: WasmEnvOutputMode>(
        contract: &mut RegistryContract<T::Hash, T::AccountId, BalanceOf<T>, T::BlockNumber>,
        input_data: Vec<u8>,
        gas_limit: Weight,
        value: BalanceOf<T>,
        escrow_account: T::AccountId,
        submitter: AuthorityId,
        requester: T::AccountId,
        gateway_id: Option<bp_runtime::ChainId>,
        gateway_abi: GatewayABIConfig,
    ) -> Result<(Vec<CircuitOutboundMessage>, Vec<u32>), &'static str> {
        let gateway_pointer = Self::retrieve_gateway_pointer(gateway_id)?;
        let gateway_inbound_protocol =
            Self::retrieve_gateway_protocol::<T>(submitter, &gateway_pointer)?;

        let maybe_input_data = match input_data.len() {
            0 => None,
            _ => Some(input_data.clone()),
        };

        let inner_exec_transfers = &mut Vec::<TransferEntry>::new();
        let constructed_outbound_messages = &mut Vec::<CircuitOutboundMessage>::new();
        let round_breakpoints = &mut Vec::<u32>::new();

        let gas_meter = &mut VVMGasMeter::<T>::new(gas_limit);
        let _deferred_storage_writes = &mut Vec::<DeferredStorageWrite>::new();
        let _call_stamps = &mut Vec::<CallStamp>::new();

        let schedule = <T as VolatileVM>::Schedule::get();

        // Create stack for multiple level of sub-calls
        // ToDo: Change to submitter
        let origin = requester.clone();
        // ToDo: Frame value equal to requested args
        let debug_message = None;

        // Here could also access and pre-load code to lazy storage of VVM
        let _executable = PrefabWasmModule::<T>::from_code(
            contract.bytes.clone(),
            &schedule,
            OM::get_run_mode(),
            gateway_id,
        )
        .map_err(|_e| "Can't decode WASM code")?;

        // if target is None if in the contracts-repository
        let target_id = gateway_id;
        let run_mode = OM::get_run_mode();

        let stack_extension = &mut StackExtension {
            escrow_account,
            requester: requester.clone(),
            storage_trie_id: contract.info.clone().unwrap().child_trie_info(),
            input_data: maybe_input_data,
            inner_exec_transfers,
            constructed_outbound_messages,
            round_breakpoints,
            gateway_inbound_protocol,
            gateway_pointer,
            gateway_abi,
            preloaded_action_descriptions: &mut contract.action_descriptions,
            target_id,
            run_mode,
        };

        let (mut stack, executable) = Stack::<T, PrefabWasmModule<T>>::new(
            FrameArgs::Call {
                dest: requester.clone(),
                cached_info: None, //  If no lazy load set Some(contract.info.clone())
            },
            origin,
            gas_meter,
            &schedule,
            value,
            debug_message,
            stack_extension,
        )
        .map_err(|_e| "Can't create VVM call stack")?;

        let _ret_out: ExecReturnValue =
            stack.run(executable, input_data).map_err(|err| err.error)?;

        // External caller should respond and if still executing locally carry on with executing next contract
        Ok((
            constructed_outbound_messages.to_vec(),
            round_breakpoints.to_vec(),
        ))
    }

    /// Returns - all messages created at this round which are immiditately available to relay to foreign consensus systems.
    pub fn pre_run_bunch_until_break<T: Config>(
        contracts: Vec<RegistryContract<T::Hash, T::AccountId, BalanceOf<T>, T::BlockNumber>>,
        escrow_account: T::AccountId,
        submitter: AuthorityId,
        requester: T::AccountId,
        value: BalanceOf<T>,
        input_data: Vec<u8>,
        gas_limit: Weight,
        gateway_id: Option<bp_runtime::ChainId>,
        gateway_abi_config: GatewayABIConfig,
    ) -> Result<(Vec<CircuitOutboundMessage>, u16), &'static str> {
        Self::preload_bunch_of_contracts::<T>(contracts.clone(), requester.clone())?;

        let constructed_outbound_messages = &mut Vec::<CircuitOutboundMessage>::new();

        let mut counter: u16 = 0;

        for mut contract in contracts {
            // ToDo: Input data can change with next loop iteration from output of the last contracts if within the same round
            let (outbound_messages_current, round_breakpoints_current) =
                Self::run_single_contract::<T, PessimisticOutputMode>(
                    &mut contract,
                    input_data.clone(),
                    gas_limit.clone(),
                    value.clone(),
                    escrow_account.clone(),
                    submitter.clone(),
                    requester.clone(),
                    gateway_id.clone(),
                    gateway_abi_config.clone(),
                )?;

            constructed_outbound_messages.extend(outbound_messages_current);

            // If the round finished return only messages produced until now
            if !round_breakpoints_current.is_empty() {
                return Ok((constructed_outbound_messages.to_vec(), counter));
            }

            counter += 1;
        }

        // All messages in that round
        Ok((constructed_outbound_messages.to_vec(), counter))
    }

    /// Pre-load is called before pre-run in order to load contracts code and info (about used space and active rent)
    /// into volatile VM (as cache).
    /// Pre-run accesses the contract code and info from the contracts-cache of VVM aka fake-storage
    pub fn preload_bunch_of_contracts<T: Config>(
        contracts: Vec<RegistryContract<T::Hash, T::AccountId, BalanceOf<T>, T::BlockNumber>>,
        account_id: T::AccountId, // purchaser
    ) -> Result<(), &'static str> {
        let schedule = <T as VolatileVM>::Schedule::get();
        // Assume contracts from on-chain repo set None as a foreign target.
        let gateway_id = None;
        // Perform syntax check and convert raw code into executables
        let executables_map = contracts
            .iter()
            .map(|contract| {
                PrefabWasmModule::<T>::from_code(
                    contract.bytes.clone(),
                    &schedule,
                    RunMode::Dry,
                    gateway_id,
                )
                .map_err(|_e| "Can't decode WASM code")
            })
            .collect::<Result<Vec<PrefabWasmModule<T>>, &'static str>>()?;

        for i in 0..contracts.len() {
            let curr_executable = executables_map[i].clone();
            let contract_info = contracts[i].info.clone();
            volatile_vm::Pallet::<T>::add_contract_code_lazy(
                curr_executable.code_hash,
                curr_executable,
                contract_info.unwrap(),
                account_id.clone(),
            )
        }

        Ok(())
    }

    fn retrieve_gateway_pointer(
        gateway_id: Option<bp_runtime::ChainId>,
    ) -> Result<GatewayPointer, &'static str> {
        match gateway_id {
            None => Ok(GatewayPointer {
                // ToDo: Setup default for Circuit equivalent to None
                id: Default::default(),
                gateway_type: GatewayType::ProgrammableExternal,
                vendor: GatewayVendor::Substrate,
            }),
            // ToDo: Lookup in pallet-xdns here to match target with vendor
            Some(gateway_id) => Ok(GatewayPointer {
                id: gateway_id,
                gateway_type: GatewayType::ProgrammableExternal,
                vendor: GatewayVendor::Substrate,
            }),
        }
    }

    /// Given a Gateway Pointer and an Authority, it returns the respective Gateway Protocol
    fn retrieve_gateway_protocol<T: crate::Config>(
        submitter_id: AuthorityId,
        gateway_pointer: &GatewayPointer,
    ) -> Result<Box<dyn GatewayInboundProtocol>, &'static str> {
        // Very dummy - replace asap with https://github.com/t3rn/t3rn/pull/87
        use crate::message_assembly::chain_generic_metadata::Metadata;
        use frame_metadata::{
            DecodeDifferent, ExtrinsicMetadata, FunctionMetadata, ModuleMetadata,
            RuntimeMetadataV13,
        };
        pub fn get_dummy_modules_with_functions() -> Vec<(&'static str, Vec<&'static str>)> {
            vec![
                ("state", vec!["call"]),
                ("state", vec!["getStorage"]),
                ("state", vec!["setStorage"]),
                ("ModuleName", vec!["FnName"]),
                ("ModuleName", vec!["FnName1"]),
                ("ModuleName", vec!["FnName2"]),
                ("ModuleName", vec!["FnName3"]),
                ("author", vec!["submitExtrinsic"]),
                ("utility", vec!["batchAll"]),
                ("system", vec!["remark"]),
                ("gateway", vec!["call"]),
                ("balances", vec!["transfer"]),
                ("gateway", vec!["getStorage"]),
                ("gateway", vec!["transfer"]),
                ("gateway", vec!["emitEvent"]),
                ("gateway", vec!["custom"]),
                ("gatewayEscrowed", vec!["callStatic"]),
                ("gatewayEscrowed", vec!["callEscrowed"]),
            ]
        }
        // Very dummy - replace asap with https://github.com/t3rn/t3rn/pull/87
        fn create_test_metadata(
            modules_with_functions: Vec<(&'static str, Vec<&'static str>)>,
        ) -> Metadata {
            let mut module_index = 0;
            let mut modules: Vec<ModuleMetadata> = vec![];

            let fn_metadata_generator = |name: &'static str| -> FunctionMetadata {
                FunctionMetadata {
                    name: DecodeDifferent::Encode(name),
                    arguments: DecodeDifferent::Decoded(vec![]),
                    documentation: DecodeDifferent::Decoded(vec![]),
                }
            };

            let module_metadata_generator = |mod_name: &'static str,
                                             mod_index: u8,
                                             functions: Vec<FunctionMetadata>|
             -> ModuleMetadata {
                ModuleMetadata {
                    index: mod_index,
                    name: DecodeDifferent::Encode(mod_name),
                    storage: None,
                    calls: Some(DecodeDifferent::Decoded(functions)),
                    event: None,
                    constants: DecodeDifferent::Decoded(vec![]),
                    errors: DecodeDifferent::Decoded(vec![]),
                }
            };

            for module in modules_with_functions {
                let (module_name, fn_names) = module;
                let functions = fn_names.into_iter().map(fn_metadata_generator).collect();
                modules.push(module_metadata_generator(
                    module_name,
                    module_index,
                    functions,
                ));
                module_index = module_index + 1;
            }

            let runtime_metadata = RuntimeMetadataV13 {
                extrinsic: ExtrinsicMetadata {
                    version: 1,
                    signed_extensions: vec![DecodeDifferent::Encode("test")],
                },
                modules: DecodeDifferent::Decoded(modules),
            };
            Metadata::new(runtime_metadata)
        }

        let mut best_gateway = pallet_xdns::Pallet::<T>::best_available(gateway_pointer.id)?;

        let genesis_hash = T::Hashing::hash(&mut best_gateway.gateway_genesis.genesis_hash);
        let runtime_version = best_gateway.gateway_genesis.runtime_version;

        Ok(Box::new(
            SubstrateGatewayProtocol::<AuthorityId, T::Hash>::new(
                // FixMe: Very dummy - replace asap with https://github.com/t3rn/t3rn/pull/87
                create_test_metadata(get_dummy_modules_with_functions()),
                runtime_version,
                genesis_hash,
                submitter_id,
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::exec_composer::{
        ExecComposer, PessimisticOutputMode, OptimisticOutputMode, RawAliveContractInfo, TEST_RUNTIME_VERSION,
    };
    use crate::mock::Test;
    use crate::{
        BalanceOf, Compose, Config, ContractActionDesc, GatewayABIConfig, GatewayGenesisConfig,
        RegistryContract, KEY_TYPE,
    };
    use codec::Encode;
    use hex_literal::hex;
    use t3rn_primitives::{GatewayExpectedOutput, GatewayType, GatewayVendor};

    use sp_core::{crypto::Pair, sr25519, Hasher};
    use frame_support::{assert_ok, weights::Weight};
    use sp_core::H256;
    use sp_io::TestExternalities;
    use sp_keystore::testing::KeyStore;
    use sp_keystore::{KeystoreExt, SyncCryptoStore};
    use sp_runtime::AccountId32;
    use volatile_vm::VolatileVM;
    use volatile_vm::wasm::{RunMode, PrefabWasmModule};
    use std::str::FromStr;

    fn make_compose_out_of_raw_wat_code<T: Config>(
        wat_string: &str,
        input_data: Vec<u8>,
        dest: T::AccountId,
        value: BalanceOf<T>,
    ) -> Compose<T::AccountId, BalanceOf<T>> {
        let wasm = match wat::parse_str(wat_string.clone()) {
            Ok(wasm) => wasm,
            Err(_err) => " invalid code str ".encode()
        };
        Compose {
            name: b"component1".to_vec(),
            code_txt: wat_string.encode(),
            exec_type: b"exec_escrow".to_vec(),
            dest,
            value,
            bytes: wasm,
            input_data,
        }
    }

    fn insert_default_xdns_record() {
        use pallet_xdns::XdnsRecord;
        pallet_xdns::XDNSRegistry::<Test>::insert(
            // Below is blake2_hash of [0, 0, 0, 0]
            H256::from_slice(&hex!(
                "11da6d1f761ddf9bdb4c9d6e5303ebd41f61858d0a5647a1a7bfe089bf921be9"
            )),
            XdnsRecord::<AccountId32>::new(
                Default::default(),
                [0, 0, 0, 0],
                Default::default(),
                GatewayVendor::Substrate,
                GatewayType::ProgrammableExternal,
                GatewayGenesisConfig {
                    modules_encoded: None,
                    signed_extension: None,
                    runtime_version: TEST_RUNTIME_VERSION,
                    genesis_hash: Default::default(),
                },
            ),
        );
    }

    fn setup_test_escrow_as_tx_signer(ext: &mut TestExternalities) -> AccountId32 {
        let keystore = KeyStore::new();
        // Insert Alice's keys
        const SURI_ALICE: &str = "//Alice";

        let key_pair_alice =
            sr25519::Pair::from_string(SURI_ALICE, None).expect("Generates key pair");
        SyncCryptoStore::insert_unknown(
            &keystore,
            KEY_TYPE,
            SURI_ALICE,
            key_pair_alice.public().as_ref(),
        )
        .expect("Inserts unknown key");

        ext.register_extension(KeystoreExt(keystore.into()));
        // Alice's account
        hex_literal::hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"].into()
    }

    fn make_registry_contract_out_of_wat<T: Config>(
        wat: &str,
        input_data: Vec<u8>,
        dest: T::AccountId,
        value: BalanceOf<T>,
    ) -> RegistryContract<T::Hash, T::AccountId, BalanceOf<T>, T::BlockNumber>
    {
        let compose = make_compose_out_of_raw_wat_code::<T>(wat, input_data, dest, value);

        RegistryContract::from_compose(
            compose.clone(),
            vec![],
            Default::default(),
            None,
            None,
            Some(RawAliveContractInfo {
                trie_id: Default::default(),
                storage_size: Default::default(),
                pair_count: Default::default(),
                code_hash: T::Hashing::hash(&compose.bytes),
                rent_allowance: Default::default(),
                rent_paid: Default::default(),
                deduct_block: Default::default(),
                last_write: Default::default(),
                _reserved: Default::default(),
            }),
            Default::default(),
        )
    }

    const INVALID_CODE : &str = r#" invalid code"#;

    const CODE_CALL: &str = r#"
(module
	;; seal_call(
	;;    callee_ptr: u32,
	;;    callee_len: u32,
	;;    gas: u64,
	;;    value_ptr: u32,
	;;    value_len: u32,
	;;    input_data_ptr: u32,
	;;    input_data_len: u32,
	;;    output_ptr: u32,
	;;    output_len_ptr: u32
	;;) -> u32
	(import "seal0" "seal_call" (func $seal_call (param i32 i32 i64 i32 i32 i32 i32 i32 i32) (result i32)))
	(import "env" "memory" (memory 1 1))
	(func (export "call")
		(drop
			(call $seal_call
				(i32.const 4)  ;; Pointer to "callee" address.
				(i32.const 32)  ;; Length of "callee" address.
				(i64.const 0)  ;; How much gas to devote for the execution. 0 = all.
				(i32.const 36) ;; Pointer to the buffer with value to transfer
				(i32.const 8)  ;; Length of the buffer with value to transfer.
				(i32.const 44) ;; Pointer to input data buffer address
				(i32.const 4)  ;; Length of input data buffer
				(i32.const 4294967295) ;; u32 max value is the sentinel value: do not copy output
				(i32.const 0) ;; Length is ignored in this case
			)
		)
	)
	(func (export "deploy"))

	;; Destination AccountId (ALICE)
	(data (i32.const 4)
		"\01\01\01\01\01\01\01\01\01\01\01\01\01\01\01\01"
		"\01\01\01\01\01\01\01\01\01\01\01\01\01\01\01\01"
	)

	;; Amount of value to transfer.
	;; Represented by u64 (8 bytes long) in little endian.
	(data (i32.const 36) "\06\00\00\00\00\00\00\00")

	(data (i32.const 44) "\01\02\03\04")
)
"#;

const WROONG_CODE_MODULE_DISPATCH_NO_FUNC: &str = r#"
(module
	(import "__unstable__" "seal_call" (func $seal_call (param i32 i32 i64 i32 i32 i32 i32 i32) (result i32)))
	(import "seal0" "seal_input" (func $seal_input (param i32 i32)))
	(import "seal0" "seal_return" (func $seal_return (param i32 i32 i32)))
	(import "env" "memory" (memory 1 1))
	(func (export "call")
		(drop
			(call $seal_call
				(i32.const 16) ;; Set MODULE_DISPATCH bit
				(i32.const 4)  ;; Pointer to "callee" address.
				(i64.const 0)  ;; How much gas to devote for the execution. 0 = all.
				(i32.const 36) ;; Pointer to the buffer with value to transfer
				(i32.const 44) ;; Pointer to input data buffer address
				(i32.const 4)  ;; Length of input data buffer
				(i32.const 4294967295) ;; u32 max value is the sentinel value: do not copy output
				(i32.const 0) ;; Length is ignored in this case
			)
		)

		;; works because the input was cloned
		(call $seal_input (i32.const 0) (i32.const 44))

		;; return the input to caller for inspection
		(call $seal_return (i32.const 0) (i32.const 0) (i32.load (i32.const 44)))
	)

	(func (export "deploy"))

	;; Destination AccountId (ALICE)
	(data (i32.const 4)
		"\01\01\01\01\01\01\01\01\01\01\01\01\01\01\01\01"
		"\01\01\01\01\01\01\01\01\01\01\01\01\01\01\01\01"
	)

	;; Amount of value to transfer.
	;; Represented by u64 (8 bytes long) in little endian.
	(data (i32.const 36) "\2A\00\00\00\00\00\00\00")

	;; The input is ignored because we forward our own input
	(data (i32.const 44) "\01\02\03\04")
)
"#;

    #[test]
    fn dry_run_succeeds_for_valid_call_contract_with_declared_foreign_target() {
        // Bob - dest
        let dest =
            AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let _gateway_id = [0 as u8; 4];
        let compose = make_compose_out_of_raw_wat_code::<Test>(CODE_CALL, vec![], dest, value);

        let mut ext = TestExternalities::new_empty();
        let escrow_account = setup_test_escrow_as_tx_signer(&mut ext);
        let _gateway_abi_config: GatewayABIConfig = Default::default();

        let account_at_foreign_target = AccountId32::from(hex!(
            "0101010101010101010101010101010101010101010101010101010101010101"
        ));
        let example_foreign_target = [1u8, 2u8, 3u8, 4u8];

        ext.execute_with(|| {
            let _submitter = crate::Pallet::<Test>::select_authority(escrow_account.clone())
                .unwrap_or_else(|_| panic!("failed to select_authority"));

            insert_default_xdns_record();

            volatile_vm::DeclaredTargets::<Test>::insert(
                account_at_foreign_target,
                example_foreign_target.clone(),
            );

            let res = ExecComposer::dry_run_single_contract::<Test>(compose);
            assert_ok!(res.clone());
            assert_eq!(
                res.unwrap().action_descriptions,
                vec![ContractActionDesc {
                    action_id: H256::from(hex!(
                        "8983f833d99e84d9dd10a9ce44549e9ba4fb831a62bd4435642ad6fa32a1da7f"
                    )),
                    target_id: Some(example_foreign_target),
                    to: Some(AccountId32::from(hex!(
                        "0101010101010101010101010101010101010101010101010101010101010101"
                    )))
                }]
            );
        });
    }

    #[test]
    fn dry_run_succeeds_for_valid_call_contract() {
        // Bob - dest
        let dest =
            AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let _gateway_id = [0 as u8; 4];
        let compose = make_compose_out_of_raw_wat_code::<Test>(CODE_CALL, vec![], dest, value);

        let mut ext = TestExternalities::new_empty();
        let escrow_account = setup_test_escrow_as_tx_signer(&mut ext);
        let _gateway_abi_config: GatewayABIConfig = Default::default();

        ext.execute_with(|| {
            insert_default_xdns_record();

            let _submitter = crate::Pallet::<Test>::select_authority(escrow_account.clone())
                .unwrap_or_else(|_| panic!("failed to select_authority"));

            let res = ExecComposer::dry_run_single_contract::<Test>(compose);

            assert_ok!(res.clone());
            assert_eq!(
                res.unwrap().action_descriptions,
                vec![ContractActionDesc {
                    action_id: H256::from(hex!(
                        "8983f833d99e84d9dd10a9ce44549e9ba4fb831a62bd4435642ad6fa32a1da7f"
                    )),
                    target_id: None,
                    to: Some(AccountId32::from(hex!(
                        "0101010101010101010101010101010101010101010101010101010101010101"
                    )))
                }]
            );
        });
    }

    #[test]
    fn dry_run_fails_for_invalid_call_contract() {
        // Bob - dest
        let dest =
            AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let _gateway_id = [0 as u8; 4];

        let compose = Compose {
            name: b"component1".to_vec(),
            code_txt: " invalid code str ".encode(),
            exec_type: b"exec_escrow".to_vec(),
            dest,
            value,
            bytes: " invalid code str ".encode(),
            input_data: vec![],
        };

        let mut ext = TestExternalities::new_empty();
        let escrow_account = setup_test_escrow_as_tx_signer(&mut ext);
        let _gateway_abi_config: GatewayABIConfig = Default::default();

        ext.execute_with(|| {
            insert_default_xdns_record();
            let _submitter = crate::Pallet::<Test>::select_authority(escrow_account.clone())
                .unwrap_or_else(|_| panic!("failed to select_authority"));
            let res = ExecComposer::dry_run_single_contract::<Test>(compose);
            assert_eq!(res, Err("Can't decode WASM code"))
        });
    }

    #[test]
    fn pre_run_produces_outbound_messages_if_declared_remote_target() {
        // Bob - requester
        let requester =
            AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let input_data = vec![];
        let gas_limit = 1726103 + 283184644 + 143915670; // gas limit for the example call
        let gateway_id = None; // on-chain contract = None as a target_id
        let compose =
            make_compose_out_of_raw_wat_code::<Test>(CODE_CALL, vec![], requester.clone(), value);

        let mut ext = TestExternalities::new_empty();
        let escrow_account = setup_test_escrow_as_tx_signer(&mut ext);
        let _gateway_abi_config: GatewayABIConfig = Default::default();

        let account_at_foreign_target = AccountId32::from(hex!(
            "0101010101010101010101010101010101010101010101010101010101010101"
        ));
        let example_foreign_target = [1u8, 2u8, 3u8, 4u8];

        ext.execute_with(|| {
            let submitter = crate::Pallet::<Test>::select_authority(escrow_account.clone())
                .unwrap_or_else(|_| panic!("failed to select_authority"));

            insert_default_xdns_record();

            volatile_vm::DeclaredTargets::<Test>::insert(
                account_at_foreign_target,
                example_foreign_target,
            );

            let _output_mode = PessimisticOutputMode::new();

            let gateway_abi_config = Default::default();
            let example_contract = ExecComposer::dry_run_single_contract::<Test>(compose).unwrap();

            let res = ExecComposer::pre_run_bunch_until_break::<Test>(
                vec![example_contract],
                escrow_account,
                submitter.clone(),
                requester,
                value,
                input_data,
                gas_limit,
                gateway_id,
                gateway_abi_config,
            );

            assert_ok!(res.clone());

            let succ_response = res.unwrap();
            let test_messages_at_this_round = succ_response.0;
            let first_message = test_messages_at_this_round[0].clone();

            crate::message_assembly::substrate_gateway_protocol::tests::assert_signed_payload(
                first_message,
                submitter.into(),
                vec![vec![4, 95], vec![1, 2, 3, 4]], // arguments
                vec![
                    GatewayExpectedOutput::Events {
                        signatures: vec![b"Call(address,value,uint64,dynamic_bytes)".to_vec()],
                    },
                    GatewayExpectedOutput::Output {
                        output: b"dynamic_bytes".to_vec(),
                    },
                ],
                vec![0, 0, 4, 95, 1, 2, 3, 4],
                vec![
                    0, 0, 4, 95, 1, 2, 3, 4, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 14, 87, 81, 192, 38,
                    229, 67, 178, 232, 171, 46, 176, 96, 153, 218, 161, 209, 229, 223, 71, 119,
                    143, 119, 135, 250, 171, 69, 205, 241, 47, 227, 168, 14, 87, 81, 192, 38, 229,
                    67, 178, 232, 171, 46, 176, 96, 153, 218, 161, 209, 229, 223, 71, 119, 143,
                    119, 135, 250, 171, 69, 205, 241, 47, 227, 168,
                ],
                "state",
                "call",
            );
        });
    }

    #[test]
    fn pre_run_recognizes_call_module_from_flags_and_fails_for_empty_names() {
        // Bob - requester
        let requester =
            AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let input_data = vec![];
        let gas_limit = Weight::MAX;
        let gateway_id = None; // on-chain contract = None as a target_id
        let compose = make_compose_out_of_raw_wat_code::<Test>(
            WROONG_CODE_MODULE_DISPATCH_NO_FUNC,
            vec![],
            requester.clone(),
            value,
        );

        let mut ext = TestExternalities::new_empty();
        let escrow_account = setup_test_escrow_as_tx_signer(&mut ext);
        let _gateway_abi_config: GatewayABIConfig = Default::default();

        let account_at_foreign_target = AccountId32::from(hex!(
            "0101010101010101010101010101010101010101010101010101010101010101"
        ));
        let example_foreign_target = [1u8, 2u8, 3u8, 4u8];

        ext.execute_with(|| {
            let submitter = crate::Pallet::<Test>::select_authority(escrow_account.clone())
                .unwrap_or_else(|_| panic!("failed to select_authority"));

            // Set the default XDNS record for default [0, 0, 0, 0] gateway
            insert_default_xdns_record();

            volatile_vm::DeclaredTargets::<Test>::insert(
                account_at_foreign_target,
                example_foreign_target,
            );

            let _output_mode = PessimisticOutputMode::new();

            let gateway_abi_config = Default::default();
            let example_contract = ExecComposer::dry_run_single_contract::<Test>(compose).unwrap();

            let res = ExecComposer::pre_run_bunch_until_break::<Test>(
                vec![example_contract],
                escrow_account,
                submitter,
                requester,
                value,
                input_data,
                gas_limit,
                gateway_id,
                gateway_abi_config,
            );

            assert_eq!(
                res,
                Err("Input < 64 doesn't allow to extract function and method names")
            )
        });
    }

    #[test]
    fn pre_run_bunch_until_break_multiple_contracts_succeeds()
    {
        let mut ext = TestExternalities::new_empty();
        let requester = AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let escrow_account = setup_test_escrow_as_tx_signer(&mut ext);

        let compose_one = make_compose_out_of_raw_wat_code::<Test>(
            CODE_CALL,
            vec![],
            requester.clone(),
            value,
        );

        let compose_two = make_compose_out_of_raw_wat_code::<Test>(
            WROONG_CODE_MODULE_DISPATCH_NO_FUNC,
            vec![],
            requester.clone(),
            value,
        );

        ext.execute_with(|| {
            let submitter = crate::Pallet::<Test>::select_authority(escrow_account.clone())
                .unwrap_or_else(|_| panic!("failed to select_authority"));
            insert_default_xdns_record();

            let contract_one = ExecComposer::dry_run_single_contract::<Test>(compose_one).unwrap();
            let contract_two = ExecComposer::dry_run_single_contract::<Test>(compose_two).unwrap();

            let res = ExecComposer::pre_run_bunch_until_break::<Test>(
                vec![contract_one,contract_two],
                escrow_account,
                submitter,
                requester,
                value,
                vec![],
                Weight::MAX,
                None,
                Default::default(),
            );

            assert_ok!(res.clone());

            let unwrapped_result = res.unwrap();
            assert_eq!(unwrapped_result.1, 2u16);
            assert_eq!(unwrapped_result.0.len(), 0);
        });
    }

    #[test]
    fn preload_bunch_of_contracts_one_contract_succeeds() {
        let mut ext = TestExternalities::new_empty();
        let requester = AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let temp_contract_one = make_registry_contract_out_of_wat::<Test>(
            CODE_CALL, 
            vec![], 
            requester.clone(), 
            value,
        );
        
        let schedule = <Test as VolatileVM>::Schedule::get();
        let executable = PrefabWasmModule::<Test>::from_code(
            temp_contract_one.bytes.clone(),
            &schedule,
            RunMode::Dry,
            None,
        ).unwrap();

        ext.execute_with(|| {
            insert_default_xdns_record();            
            let res = ExecComposer::preload_bunch_of_contracts::<Test>(vec![temp_contract_one], Default::default());

            assert_ok!(res);
            // If we are able to unwrap, it means the contract was inserted.
            let _fetched_contract = volatile_vm::Pallet::<Test>::get_contract_code_lazy(executable.code_hash).unwrap();
        });
    }

    #[test]
    fn preload_bunch_of_contracts_multiple_contract_succeeds() {
        let mut ext = TestExternalities::new_empty();
        let requester = AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let temp_contract_one = make_registry_contract_out_of_wat::<Test>(
            CODE_CALL, 
            vec![], 
            requester.clone(), 
            value,
        );
        let temp_contract_two = make_registry_contract_out_of_wat::<Test>(
            CODE_CALL, 
            vec![], 
            requester.clone(), 
            value,
        );
        ext.execute_with(|| {
            insert_default_xdns_record();

            let res = ExecComposer::preload_bunch_of_contracts::<Test>(vec![temp_contract_one, temp_contract_two], Default::default());

            assert_ok!(res);
        });
    }

    #[test]
    fn preload_bunch_of_contracts_one_contract_fails() {
        let mut ext = TestExternalities::new_empty();
        let requester = AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let temp_contract_one = make_registry_contract_out_of_wat::<Test>(
            INVALID_CODE, 
            vec![], 
            requester.clone(), 
            value,
        );

        ext.execute_with(|| {
            insert_default_xdns_record();
            
            let res = ExecComposer::preload_bunch_of_contracts::<Test>(vec![temp_contract_one], Default::default());

            assert_eq!(res, Err("Can't decode WASM code"));
        });
    }

    #[test]
    fn preload_bunch_of_contracts_multiple_contract_fails() {
        let mut ext = TestExternalities::new_empty();
        let requester = AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let temp_contract_one = make_registry_contract_out_of_wat::<Test>(
            CODE_CALL, 
            vec![], 
            requester.clone(), 
            value,
        );
        let temp_contract_two = make_registry_contract_out_of_wat::<Test>(
            INVALID_CODE, 
            vec![], 
            requester.clone(), 
            value,
        );

        ext.execute_with(|| {
            insert_default_xdns_record();            
            let res = ExecComposer::preload_bunch_of_contracts::<Test>(vec![temp_contract_one, temp_contract_two], Default::default());

            assert_eq!(res, Err("Can't decode WASM code"));
        });
    }

    #[test]
    fn run_single_contract_fails_stack_error()
    {
        let mut ext = TestExternalities::new_empty();
        let requester = AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);        
        let mut temp_contract_one = make_registry_contract_out_of_wat::<Test>(
            CODE_CALL, 
            vec![], 
            requester.clone(), 
            value,
        );

        ext.execute_with(|| {
            insert_default_xdns_record();
            let res = ExecComposer::run_single_contract::<Test, OptimisticOutputMode>(
                &mut temp_contract_one,
                Default::default(),
                Weight::MAX,
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),// requester
                Default::default(),
                Default::default(),
            );

            assert_eq!(res, Err("Can't create VVM call stack"));
        });
    }

    #[test]
    fn run_single_contract_fails_xdns_error()
    {
        let mut ext = TestExternalities::new_empty();
        let requester = AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);        
        let mut temp_contract_one = make_registry_contract_out_of_wat::<Test>(
            CODE_CALL, 
            vec![], 
            requester.clone(), 
            value,
        );

        ext.execute_with(|| {
            // This comment line is intentional
            // Not inserting xdns record to replicate this failure
            // insert_default_xdns_record();

            let res = ExecComposer::run_single_contract::<Test, OptimisticOutputMode>(
                &mut temp_contract_one,
                Default::default(),
                Weight::MAX,
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            );
            
            assert_eq!(res, Err("Xdns record not found"));
        });
    }

    #[test]
    fn run_single_contract_fails_wasm_parse_error()
    {
        let mut ext = TestExternalities::new_empty();
        let requester = AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let mut temp_contract_one = make_registry_contract_out_of_wat::<Test>(
            INVALID_CODE, 
            vec![], 
            requester.clone(), 
            value,
        );
        ext.execute_with(|| {
            insert_default_xdns_record();
            let res = ExecComposer::run_single_contract::<Test, OptimisticOutputMode>(
                &mut temp_contract_one,
                Default::default(),
                Weight::MAX,
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            );

            assert_eq!(res, Err("Can't decode WASM code"));
        });
    }

    #[test]
    #[ignore = "Please implement proper assertions when retrieve_gateway_protocol is properly implemented"]
    fn run_single_contract_fails_retrieve_gateway_protocol_error()
    {
        let mut ext = TestExternalities::new_empty();
        let requester = AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let mut temp_contract_one = make_registry_contract_out_of_wat::<Test>(
            CODE_CALL, 
            vec![], 
            requester.clone(), 
            value,
        );

        ext.execute_with(|| {
            insert_default_xdns_record();

            let res = ExecComposer::run_single_contract::<Test, OptimisticOutputMode>(
                &mut temp_contract_one,
                Default::default(),
                Weight::MAX,
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            );
            
            assert_eq!(res, res);
        });
    }

    #[test]
    fn run_single_contract_succeeds()
    {
        let mut ext = TestExternalities::new_empty();
        let requester = AccountId32::from_str("5G9VdMwXvzza9pS8qE8ZHJk3CheHW9uucBn9ngW4C1gmmzpv").unwrap();
        let value = BalanceOf::<Test>::from(0u32);
        let mut temp_contract_one = make_registry_contract_out_of_wat::<Test>(
            CODE_CALL, 
            vec![], 
            requester.clone(), 
            value,
        );

        ext.execute_with(|| {
            insert_default_xdns_record();
            let preload_response = ExecComposer::preload_bunch_of_contracts::<Test>(vec![temp_contract_one.clone()], Default::default());
            assert_ok!(preload_response);

            let res = ExecComposer::run_single_contract::<Test, OptimisticOutputMode>(
                &mut temp_contract_one,
                Default::default(),
                Weight::MAX,
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),// requester
                Default::default(),
                Default::default(),
            );
            assert_ok!(res.clone());
        });
    }
}
