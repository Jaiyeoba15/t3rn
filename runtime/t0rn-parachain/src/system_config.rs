use crate::{accounts_config::AccountManagerCurrencyAdapter, Hash as HashPrimitive, *};
use frame_support::{
    parameter_types,
    traits::{
        fungibles::{Balanced, CreditOf},
        ConstU32, ConstU8, Contains, OffchainWorker, OnFinalize, OnIdle, OnInitialize,
        OnRuntimeUpgrade,
    },
    weights::IdentityFee,
};
use pallet_asset_tx_payment::HandleCredit;
use polkadot_runtime_common::SlowAdjustingFeeUpdate;
use sp_runtime::traits::{BlakeTwo256, ConvertInto, Zero};

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
    pub const SS58Prefix: u16 = 42;
}

// Configure FRAME pallets to include in runtime.
impl frame_system::Config for Runtime {
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The basic call filter to use in dispatchable.
    // type BaseCallFilter = MaintenanceMode; // todo: fix MaintananceMode compilation for v39 - XCMExecuteTransact trait unimplemented error
    type BaseCallFilter = frame_support::traits::Everything;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// The maximum length of a block (in bytes).
    type BlockLength = circuit_runtime_types::RuntimeBlockLength;
    /// The index type for blocks.
    type BlockNumber = circuit_runtime_types::BlockNumber;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = circuit_runtime_types::RuntimeBlockWeights;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// The type for hashing blocks and tries.
    type Hash = HashPrimitive;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = AccountIdLookup<AccountId, ()>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// The set code logic, just the default since we're not a parachain.
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    /// Converts a module to the index of the module in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type PalletInfo = PalletInfo;
    /// The aggregated dispatch type that is available for extrinsics.
    type RuntimeCall = RuntimeCall;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    /// The ubiquitous origin type.
    type RuntimeOrigin = RuntimeOrigin;
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = ();
    /// Version of the runtime.
    type Version = Version;
}

impl pallet_randomness_collective_flip::Config for Runtime {}

impl pallet_timestamp::Config for Runtime {
    type MinimumPeriod = MinimumPeriod;
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Aura;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1_u128;
}

impl pallet_balances::Config for Runtime {
    type AccountStore = System;
    /// The type for recording an account's balance.
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const TransactionByteFee: Balance = 1;
}

impl pallet_transaction_payment::Config for Runtime {
    type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type OnChargeTransaction = AccountManagerCurrencyAdapter<Balances, ()>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type RuntimeEvent = RuntimeEvent;
    type WeightToFee = IdentityFee<Balance>;
}

/// A `HandleCredit` implementation that transfers 80% of the fees to the
/// block author and 20% to the treasury. Will drop and burn the assets
/// in case the transfer fails.
pub struct CreditToBlockAuthor;
impl HandleCredit<AccountId, Assets> for CreditToBlockAuthor {
    fn handle_credit(credit: CreditOf<AccountId, Assets>) {
        if let Some(author) = pallet_authorship::Pallet::<Runtime>::author() {
            let author_credit = credit
                .peek()
                .saturating_mul(80_u32.into())
                .saturating_div(<u32 as Into<Balance>>::into(100_u32));
            let (author_cut, treasury_cut) = credit.split(author_credit);
            // Drop the result which will trigger the `OnDrop` of the imbalance in case of error.
            match Assets::resolve(&author, author_cut) {
                Ok(_) => (),
                Err(_err) => {
                    log::error!("Failed to credit block author");
                },
            }
            match Assets::resolve(&Treasury::account_id(), treasury_cut) {
                Ok(_) => (),
                Err(_err) => {
                    log::error!("Failed to credit treasury");
                },
            }
        }
    }
}

impl pallet_sudo::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
}

impl pallet_utility::Config for Runtime {
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

pub struct BaseCallFilter;
impl Contains<RuntimeCall> for BaseCallFilter {
    fn contains(c: &RuntimeCall) -> bool {
        match c {
            // System support
            RuntimeCall::System(_) => true,
            RuntimeCall::ParachainSystem(_) => true,
            RuntimeCall::Timestamp(_) => true,
            RuntimeCall::Preimage(_) => true,
            RuntimeCall::Scheduler(_) => true,
            RuntimeCall::Utility(_) => true,
            RuntimeCall::Identity(_) => true,
            // Monetary
            RuntimeCall::Balances(_) => true,
            RuntimeCall::Assets(_) => true,
            RuntimeCall::Treasury(_) => true,
            RuntimeCall::AccountManager(method) => matches!(
                method,
                pallet_account_manager::Call::deposit { .. }
                    | pallet_account_manager::Call::finalize { .. }
            ),
            // Collator support
            RuntimeCall::CollatorSelection(_) => true,
            RuntimeCall::Session(_) => true,
            // XCM helpers
            RuntimeCall::XcmpQueue(_) => true,
            RuntimeCall::PolkadotXcm(_) => false,
            RuntimeCall::DmpQueue(_) => true,
            // RuntimeCall::XBIPortal(_) => true,
            RuntimeCall::AssetRegistry(_) => true,
            // t3rn pallets
            RuntimeCall::XDNS(_) => true,
            RuntimeCall::ContractsRegistry(method) => matches!(
                method,
                pallet_contracts_registry::Call::add_new_contract { .. }
                    | pallet_contracts_registry::Call::purge { .. }
            ),
            RuntimeCall::Circuit(method) => matches!(
                method,
                pallet_circuit::Call::on_local_trigger { .. }
                    | pallet_circuit::Call::on_xcm_trigger { .. }
                    | pallet_circuit::Call::on_remote_gateway_trigger { .. }
                    | pallet_circuit::Call::cancel_xtx { .. }
                    | pallet_circuit::Call::revert { .. }
                    | pallet_circuit::Call::on_extrinsic_trigger { .. }
                    | pallet_circuit::Call::bid_sfx { .. }
                    | pallet_circuit::Call::confirm_side_effect { .. }
            ),
            RuntimeCall::Attesters(_) => true,
            // 3VM
            RuntimeCall::ThreeVm(_) => false,
            RuntimeCall::Contracts(method) => matches!(
                method,
                pallet_3vm_contracts::Call::call { .. }
                    | pallet_3vm_contracts::Call::instantiate_with_code { .. }
                    | pallet_3vm_contracts::Call::instantiate { .. }
                    | pallet_3vm_contracts::Call::upload_code { .. }
                    | pallet_3vm_contracts::Call::remove_code { .. }
            ),
            RuntimeCall::Evm(method) => matches!(
                method,
                pallet_3vm_evm::Call::withdraw { .. }
                    | pallet_3vm_evm::Call::call { .. }
                    | pallet_3vm_evm::Call::create { .. }
                    | pallet_3vm_evm::Call::create2 { .. } // | pallet_3vm_evm::Call::claim { .. } TODO: wheres this gone
            ),
            // Portal
            RuntimeCall::Portal(_) => true,
            _ => true,
        }
    }
}

/// Maintenance mode Call filter
///
/// For maintenance mode, we disallow everything
pub struct MaintenanceFilter;
impl Contains<RuntimeCall> for MaintenanceFilter {
    fn contains(c: &RuntimeCall) -> bool {
        match c {
            // We want to make calls to the system and scheduler pallets
            RuntimeCall::System(_) => true,
            RuntimeCall::Scheduler(_) => true,
            // Sometimes scheduler/system calls require utility calls, particularly batch
            RuntimeCall::Utility(_) => true,
            // We dont manually control these so likely we dont want to block them during maintenance mode
            RuntimeCall::Balances(_) => true,
            RuntimeCall::Assets(_) => true,
            // We wanna be able to make sudo calls in maintenance mode just incase
            RuntimeCall::Sudo(_) => true,
            RuntimeCall::ParachainSystem(_) => true,
            RuntimeCall::Timestamp(_) => true,
            RuntimeCall::Session(_) => true,
            RuntimeCall::RococoBridge(_) => true,
            RuntimeCall::KusamaBridge(_) => true,
            RuntimeCall::PolkadotBridge(_) => true,
            RuntimeCall::EthereumBridge(_) => true,
            RuntimeCall::SepoliaBridge(_) => true,
            #[allow(unreachable_patterns)] // We need this as an accidental catchall
            _ => false,
        }
    }
}

/// Hooks to run when in Maintenance Mode
pub struct MaintenanceHooks;

impl OnInitialize<BlockNumber> for MaintenanceHooks {
    fn on_initialize(n: BlockNumber) -> frame_support::weights::Weight {
        AllPalletsWithSystem::on_initialize(n)
    }
}

/// Only two pallets use `on_idle`: xcmp and dmp queues.
/// Empty on_idle, in case we want the pallets to execute it, should be provided here.
impl OnIdle<BlockNumber> for MaintenanceHooks {
    fn on_idle(_n: BlockNumber, _max_weight: Weight) -> Weight {
        Weight::zero()
    }
}

impl OnRuntimeUpgrade for MaintenanceHooks {
    fn on_runtime_upgrade() -> Weight {
        AllPalletsWithSystem::on_runtime_upgrade()
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
        AllPalletsWithSystem::pre_upgrade()
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
        AllPalletsWithSystem::post_upgrade()
    }
}

impl OnFinalize<BlockNumber> for MaintenanceHooks {
    fn on_finalize(n: BlockNumber) {
        AllPalletsWithSystem::on_finalize(n)
    }
}

impl OffchainWorker<BlockNumber> for MaintenanceHooks {
    fn offchain_worker(n: BlockNumber) {
        AllPalletsWithSystem::offchain_worker(n)
    }
}

impl pallet_maintenance_mode::Config for Runtime {
    type MaintenanceCallFilter = MaintenanceFilter;
    type MaintenanceExecutiveHooks = AllPalletsWithSystem;
    type MaintenanceOrigin = EnsureRoot<AccountId>;
    type NormalCallFilter = BaseCallFilter;
    type NormalExecutiveHooks = AllPalletsWithSystem;
    type RuntimeEvent = RuntimeEvent;
}

#[cfg(test)]
mod tests {
    use super::*;
    use codec::Compact;
    use pallet_circuit::Outcome;
    use sp_runtime::{AccountId32, MultiAddress};
    use t3rn_primitives::claimable::BenefitSource;

    /// Test that the calls that are allowed in base mode can be called
    #[test]
    fn base_filter_works_with_allowed_and_disallowed_calls() {
        // System support
        let call = frame_system::Call::remark { remark: vec![] }.into();
        assert!(BaseCallFilter::contains(&call));

        let call = pallet_timestamp::Call::set { now: 0 }.into();
        assert!(BaseCallFilter::contains(&call));

        let prev_call = pallet_identity::Call::add_registrar {
            account: MultiAddress::Address32([0; 32]),
        }
        .into();
        assert!(BaseCallFilter::contains(&prev_call));

        // Monetary
        let call = pallet_account_manager::Call::finalize {
            charge_id: Default::default(),
            outcome: Outcome::Commit,
            maybe_recipient: Default::default(),
            maybe_actual_fees: Default::default(),
        }
        .into();
        assert!(BaseCallFilter::contains(&call));

        let call = pallet_xdns::Call::purge_gateway_record {
            requester: AccountId32::new([0; 32]),
            gateway_id: Default::default(),
        }
        .into();
        assert!(BaseCallFilter::contains(&call));

        let call = pallet_contracts_registry::Call::purge {
            requester: AccountId32::new([0; 32]),
            contract_id: Default::default(),
        }
        .into();
        assert!(BaseCallFilter::contains(&call));

        let call = pallet_circuit::Call::bid_sfx {
            sfx_id: Default::default(),
            bid_amount: 0,
        }
        .into();
        assert!(BaseCallFilter::contains(&call));

        let call = pallet_balances::Call::transfer {
            dest: MultiAddress::Address32([0; 32]),
            value: 0,
        }
        .into();
        assert!(BaseCallFilter::contains(&call));

        let call = pallet_assets::Call::create {
            id: Default::default(),
            admin: MultiAddress::Address32([0; 32]),
            min_balance: 0,
        }
        .into();
        assert!(BaseCallFilter::contains(&call));

        let call = pallet_treasury::Call::<Runtime, ()>::propose_spend {
            value: 0,
            beneficiary: MultiAddress::Address32([0; 32]),
        }
        .into();
        assert!(BaseCallFilter::contains(&call));

        // 3VM
        let call = pallet_3vm_evm::Call::withdraw {
            address: Default::default(),
            value: 0,
        }
        .into();
        assert!(BaseCallFilter::contains(&call));

        let call = pallet_3vm_contracts::Call::call {
            dest: MultiAddress::Address32([0; 32]),
            data: vec![],
            gas_limit: 0.into(),
            value: 0,
            storage_deposit_limit: Some(Compact(0)),
        }
        .into();
        assert!(BaseCallFilter::contains(&call));

        // Admin
        let call = pallet_sudo::Call::sudo {
            call: Box::new(frame_system::Call::remark { remark: vec![] }.into()),
        }
        .into();
        assert!(BaseCallFilter::contains(&call));

        let call = pallet_portal::Call::register_gateway {
            gateway_id: Default::default(),
            token_id: Default::default(),
            verification_vendor: Default::default(),
            execution_vendor: Default::default(),
            codec: Default::default(),
            registrant: Default::default(),
            escrow_account: Default::default(),
            allowed_side_effects: Default::default(),
            token_props: Default::default(),
            encoded_registration_data: Default::default(),
        }
        .into();
        assert!(BaseCallFilter::contains(&call));

        // Anything else
        // assert!(!BaseCallFilter::contains(_));
    }

    /// Test that the calls that are not allowed in maintenance mode cannot be called
    #[test]
    fn maintenance_filter_works_with_allowed_and_disallowed_calls() {
        let call = frame_system::Call::remark { remark: vec![] }.into();
        assert!(MaintenanceFilter::contains(&call));

        let call = pallet_utility::Call::as_derivative {
            index: 0,
            call: Box::new(call),
        }
        .into();
        assert!(MaintenanceFilter::contains(&call));

        let call = pallet_balances::Call::transfer {
            dest: MultiAddress::Address32([0; 32]),
            value: 0,
        }
        .into();
        assert!(MaintenanceFilter::contains(&call));

        let call = pallet_assets::Call::create {
            id: Default::default(),
            admin: MultiAddress::Address32([0; 32]),
            min_balance: 0,
        }
        .into();
        assert!(MaintenanceFilter::contains(&call));

        let call = pallet_timestamp::Call::set { now: 0 }.into();
        assert!(MaintenanceFilter::contains(&call));

        let call = pallet_sudo::Call::sudo {
            call: Box::new(frame_system::Call::remark { remark: vec![] }.into()),
        }
        .into();
        assert!(MaintenanceFilter::contains(&call));

        let call = pallet_3vm_evm::Call::withdraw {
            address: Default::default(),
            value: 0,
        }
        .into();
        assert!(!MaintenanceFilter::contains(&call));

        let call = pallet_identity::Call::add_registrar {
            account: MultiAddress::Address32([0; 32]),
        }
        .into();
        assert!(!MaintenanceFilter::contains(&call));

        let call = pallet_treasury::Call::<Runtime, ()>::reject_proposal { proposal_id: 0 }.into();
        assert!(!MaintenanceFilter::contains(&call));

        let call = pallet_account_manager::Call::deposit {
            charge_id: Default::default(),
            payee: AccountId32::new([0; 32]),
            charge_fee: 0,
            offered_reward: 0,
            source: BenefitSource::BootstrapPool,
            role: pallet_circuit::CircuitRole::Relayer,
            recipient: Default::default(),
            maybe_asset_id: Default::default(),
        }
        .into();
        assert!(!MaintenanceFilter::contains(&call));

        let call = pallet_xdns::Call::purge_gateway_record {
            requester: AccountId32::new([0; 32]),
            gateway_id: Default::default(),
        }
        .into();
        assert!(!MaintenanceFilter::contains(&call));

        let call = pallet_contracts_registry::Call::purge {
            requester: AccountId32::new([0; 32]),
            contract_id: Default::default(),
        }
        .into();
        assert!(!MaintenanceFilter::contains(&call));

        let call = pallet_circuit::Call::bid_sfx {
            sfx_id: Default::default(),
            bid_amount: 0,
        }
        .into();
        assert!(!MaintenanceFilter::contains(&call));

        // Anything else
        // assert!(!MaintenanceFilter::contains(_));
    }
}
