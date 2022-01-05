// This file is part of Substrate.

// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License")
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! <!-- markdown-link-check-disable -->
//!
//! ## Overview
//!
//! Circuit MVP
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};

use frame_system::ensure_signed;
use frame_system::offchain::{SignedPayload, SigningTypes};
use frame_system::pallet_prelude::OriginFor;

use sp_runtime::RuntimeDebug;

pub use t3rn_primitives::{
    abi::{GatewayABIConfig, HasherAlgo as HA},
    side_effect::{ConfirmedSideEffect, FullSideEffect, SideEffect, SideEffectId},
    transfers::BalanceOf,
    volatile::LocalState,
    xtx::{Xtx, XtxId},
    GatewayType, *,
};
use t3rn_protocol::side_effects::loader::{SideEffectsLazyLoader, UniversalSideEffectsProtocol};
pub use t3rn_protocol::{circuit_inbound::StepConfirmation, merklize::*};

use sp_runtime::traits::{Saturating, Zero};

use sp_std::fmt::Debug;

use frame_support::traits::{Currency, ExistenceRequirement::AllowDeath};

use sp_runtime::KeyTypeId;

pub type Bytes = sp_core::Bytes;

pub use pallet::*;

#[cfg(test)]
pub mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod mock;

pub mod weights;

pub mod state;

pub use t3rn_protocol::side_effects::protocol::SideEffectConfirmationProtocol;

/// Defines application identifier for crypto keys of this module.
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When offchain worker is signing transactions it's going to request keys of type
/// `KeyTypeId` from the keystore and use the ones it finds to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"circ");

pub type SystemHashing<T> = <T as frame_system::Config>::Hashing;
use crate::state::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_support::traits::Get;
    use frame_support::PalletId;
    use frame_system::pallet_prelude::*;

    pub use crate::weights::WeightInfo;

    /// Current Circuit's context of active insurance deposits
    ///
    #[pallet::storage]
    #[pallet::getter(fn get_insurance_deposits)]
    pub type InsuranceDeposits<T> = StorageDoubleMap<
        _,
        Identity,
        XExecSignalId<T>,
        Identity,
        SideEffectId<T>,
        InsuranceDeposit<
            <T as frame_system::Config>::AccountId,
            <T as frame_system::Config>::BlockNumber,
            BalanceOf<T>,
        >,
        ValueQuery,
    >;

    /// Current Circuit's context of active transactions
    ///
    #[pallet::storage]
    #[pallet::getter(fn get_x_exec_signals)]
    pub type XExecSignals<T> = StorageMap<
        _,
        Identity,
        XExecSignalId<T>,
        XExecSignal<
            <T as frame_system::Config>::AccountId,
            <T as frame_system::Config>::BlockNumber,
            BalanceOf<T>,
        >,
        ValueQuery,
    >;

    /// Current Circuit's context of active full side effects (requested + confirmation proofs)
    #[pallet::storage]
    #[pallet::getter(fn get_xtx_insurance_links)]
    pub type XtxInsuranceLinks<T> =
        StorageMap<_, Identity, XExecSignalId<T>, Vec<SideEffectId<T>>, ValueQuery>;

    /// Current Circuit's context of active full side effects (requested + confirmation proofs)
    #[pallet::storage]
    #[pallet::getter(fn get_full_side_effects)]
    pub type FullSideEffects<T> = StorageMap<
        _,
        Identity,
        XExecSignalId<T>,
        Vec<
            Vec<
                FullSideEffect<
                    <T as frame_system::Config>::AccountId,
                    <T as frame_system::Config>::BlockNumber,
                    BalanceOf<T>,
                >,
            >,
        >,
        ValueQuery,
    >;
    //     /// The currently active composable transactions, indexed according to the order of creation.
    //     #[pallet::storage]
    //     pub type ActiveXtxMap<T> = StorageMap<
    //         _,
    //         Blake2_128Concat,
    //         XtxId<T>,
    //         Xtx<
    //             <T as frame_system::Config>::AccountId,
    //             <T as frame_system::Config>::BlockNumber,
    //             BalanceOf<T>,
    //         >,
    //         OptionQuery,
    //     >;

    /// This pallet's configuration trait
    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + pallet_balances::Config
        // + pallet_contracts_registry::Config
        // + pallet_exec_delivery::Config
        + pallet_xdns::Config
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The overarching dispatch call type.
        type Call: From<Call<Self>>;

        type WeightInfo: weights::WeightInfo;

        type PalletId: Get<PalletId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        // `on_initialize` is executed at the beginning of the block before any extrinsic are
        // dispatched.
        //
        // This function must return the weight consumed by `on_initialize` and `on_finalize`.
        fn on_initialize(_n: T::BlockNumber) -> Weight {
            // Anything that needs to be done at the start of the block.
            // We don't do anything here.
            0
        }

        fn on_finalize(_n: T::BlockNumber) {
            // We don't do anything here.

            // if module block number
            // x-t3rn#4: Go over open Xtx and cancel if necessary
        }

        // A runtime code run after every block and have access to extended set of APIs.
        //
        // For instance you can generate extrinsics for the upcoming produced block.
        fn offchain_worker(_n: T::BlockNumber) {
            // We don't do anything here.
            // but we could dispatch extrinsic (transaction/unsigned/inherent) using
            // sp_io::submit_extrinsic
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Used by other pallets that want to create the exec order
        #[pallet::weight(<T as pallet::Config>::WeightInfo::on_local_trigger())]
        pub fn on_local_trigger(_origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // ToDo: pallet-circuit x-t3rn# : Authorize : Check TriggerAuthRights for local triggers

            // ToDo: pallet-circuit x-t3rn# : Validate : insurance for reversible side effects if necessary

            // ToDo: pallet-circuit x-t3rn# : Charge : fees

            // ToDo: pallet-circuit x-t3rn# : Design Storage - Propose and organise the state of Circuit. Specifically inspect the state updates in between ExecDelivery + Circuit

            // ToDo: pallet-circuit x-t3rn# : Setup : Create new Xtx and modify state - get LocalState (for Xtx) + GlobalState (for Circuit) for exec

            // ToDo: pallet-circuit x-t3rn# : Emit : Connect to ExecDelivery::submit_side_effect_temp( )

            // ToDo: pallet-circuit x-t3rn# : Cancel : Execution on timeout
            // ToDo: pallet-circuit x-t3rn# : Apply - Submission : Apply changes to storage after Submit has passed
            // ToDo: pallet-circuit x-t3rn# : Apply - Confirmation : Apply changes to storage after Confirmation has passed
            // ToDo: pallet-circuit x-t3rn# : Apply - Revert : Apply changes to storage after Revert has been proven
            // ToDo: pallet-circuit x-t3rn# : Apply - Commit : Apply changes to storage after Successfully Commit has been requested
            // ToDo: pallet-circuit x-t3rn# : Apply - Cancel : Apply changes to storage after the timeout has passed

            unimplemented!();
        }

        #[pallet::weight(<T as pallet::Config>::WeightInfo::on_local_trigger())]
        pub fn on_xcm_trigger(_origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // ToDo: Check TriggerAuthRights for local triggers
            unimplemented!();
        }

        #[pallet::weight(<T as pallet::Config>::WeightInfo::on_local_trigger())]
        pub fn on_remote_gateway_trigger(_origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            // ToDo: Check TriggerAuthRights for remote gateway triggers
            unimplemented!();
        }

        #[pallet::weight(<T as pallet::Config>::WeightInfo::on_local_trigger())]
        pub fn on_extrinsics_trigger(
            origin: OriginFor<T>,
            side_effects: Vec<SideEffect<T::AccountId, T::BlockNumber, BalanceOf<T>>>,
            fee: BalanceOf<T>,
            sequential: bool,
        ) -> DispatchResultWithPostInfo {
            // Authorize: Retrieve sender of the transaction.
            let requester = Self::authorize(origin, CircuitRole::Requester)?;

            // Charge: Ensure can afford
            let _available_trn_balance = Self::charge(&requester, fee)?;
            // Setup: new xtx context
            let mut local_xtx_ctx: LocalXtxCtx<T> =
                Self::setup(CircuitStatus::Requested, &requester, fee, None)?;
            // Validate: Side Effects
            let full_side_effects =
                Self::validate(&side_effects, &mut local_xtx_ctx, &requester, sequential)?;
            // Apply: all necessary changes to state in 1 go
            Self::apply(&mut local_xtx_ctx, &full_side_effects, None)?;
            // Emit: From Circuit events
            Self::emit(local_xtx_ctx.xtx_id, &requester, &side_effects, sequential);

            Ok(().into())
        }

        #[pallet::weight(<T as pallet::Config>::WeightInfo::on_local_trigger())]
        pub fn bond_insurance_deposit(
            origin: OriginFor<T>, // Active relayer
            xtx_id: XExecSignalId<T>,
            side_effect_id: SideEffectId<T>,
        ) -> DispatchResultWithPostInfo {
            // Authorize: Retrieve sender of the transaction.
            let relayer = Self::authorize(origin, CircuitRole::Relayer)?;

            // Setup: retrieve local xtx context
            let mut local_xtx_ctx: LocalXtxCtx<T> = Self::setup(
                CircuitStatus::PendingInsurance,
                &relayer,
                Zero::zero(),
                Some(xtx_id),
            )?;

            let (_id, mut insurance_deposit) = if let Some(ref mut insurance_tuple) = local_xtx_ctx
                .insurance_deposits
                .iter_mut()
                .find(|(id, _)| *id == side_effect_id)
            {
                Ok(insurance_tuple.clone())
            } else {
                Err(Error::<T>::InsuranceBondNotRequired)
            }?;

            // Charge: Ensure can afford
            Self::charge(&relayer, insurance_deposit.insurance)?;

            insurance_deposit.bonded_relayer = Some(relayer);
            // ToDo: Consider removing status from insurance_deposit since redundant with relayer: Option<Relayer>
            insurance_deposit.status = CircuitStatus::Bonded;

            // Apply: all necessary changes to state in 1 go
            Self::apply(
                &mut local_xtx_ctx,
                &vec![],
                Some((side_effect_id, insurance_deposit)),
            )?;
            // Emit: From Circuit events
            // Self::deposit_event(InsuredTransfer(relayer,insurance_deposit.requester,insurance_deposit.insurance))

            Ok(().into())
        }
    }

    /// Events for the pallet.
    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        // Listeners - users + SDK + UI to know whether their request is accepted for exec and pending
        XTransactionReceivedForExec(XExecSignalId<T>),
        // Listeners - executioners/relayers to know new challenges and perform offline risk/reward calc
        //  of whether side effect is worth picking up
        NewSideEffectsAvailable(
            <T as frame_system::Config>::AccountId,
            XExecSignalId<T>,
            Vec<
                SideEffect<
                    <T as frame_system::Config>::AccountId,
                    <T as frame_system::Config>::BlockNumber,
                    BalanceOf<T>,
                >,
            >,
        ),
    }

    #[pallet::error]
    pub enum Error<T> {
        RequesterNotEnoughBalance,
        ChargingTransferFailed,
        InsuranceBondNotRequired,
        InsuranceBondAlreadyDeposited,
        SetupFailed,
        ApplyFailed,
        DeterminedForbiddenXtxStatus,
        UnsupportedRole,
    }
}

pub fn get_xtx_status() {}
/// Payload used by this example crate to hold price
/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Payload<Public, BlockNumber> {
    block_number: BlockNumber,
    public: Public,
}

impl<T: SigningTypes> SignedPayload<T> for Payload<T::Public, T::BlockNumber> {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

impl<T: Config> Pallet<T> {
    fn setup(
        current_status: CircuitStatus,
        requester: &T::AccountId,
        reward: BalanceOf<T>,
        xtx_id: Option<XExecSignalId<T>>,
    ) -> Result<LocalXtxCtx<T>, Error<T>> {
        match current_status {
            CircuitStatus::Requested => {
                if let Some(id) = xtx_id {
                    if <Self as Store>::XExecSignals::contains_key(id) {
                        return Err(Error::<T>::SetupFailed);
                    }
                }
                // ToDo: Introduce default timeout + delay
                let (timeouts_at, delay_steps_at): (
                    Option<T::BlockNumber>,
                    Option<Vec<T::BlockNumber>>,
                ) = (None, None);

                let (x_exec_signal_id, x_exec_signal) =
                    XExecSignal::<T::AccountId, T::BlockNumber, BalanceOf<T>>::setup_fresh::<T>(
                        requester,
                        timeouts_at,
                        delay_steps_at,
                        Some(reward),
                    );

                Ok(LocalXtxCtx {
                    local_state: LocalState::new(),
                    use_protocol: UniversalSideEffectsProtocol::new(),
                    xtx_id: x_exec_signal_id,
                    xtx: x_exec_signal,
                    insurance_deposits: vec![],
                })
            }
            CircuitStatus::PendingInsurance => {
                if let Some(id) = xtx_id {
                    if !<Self as Store>::XExecSignals::contains_key(id) {
                        return Err(Error::<T>::SetupFailed);
                    }
                    let xtx = <Self as Store>::XExecSignals::get(id);
                    if xtx.status != CircuitStatus::PendingInsurance {
                        return Err(Error::<T>::SetupFailed);
                    }
                    let insurance_deposits = <Self as Store>::XtxInsuranceLinks::get(id)
                        .iter()
                        .map(|&se_id| (se_id, <Self as Store>::InsuranceDeposits::get(id, se_id)))
                        .collect::<Vec<(
                            SideEffectId<T>,
                            InsuranceDeposit<T::AccountId, T::BlockNumber, BalanceOf<T>>,
                        )>>();

                    Ok(LocalXtxCtx {
                        local_state: LocalState::new(),
                        use_protocol: UniversalSideEffectsProtocol::new(),
                        xtx_id: id,
                        xtx,
                        insurance_deposits,
                    })
                } else {
                    Err(Error::<T>::SetupFailed)
                }
            }
            _ => unimplemented!(),
        }
    }

    fn apply(
        local_ctx: &mut LocalXtxCtx<T>,
        full_ordered_side_effects: &Vec<
            Vec<FullSideEffect<T::AccountId, T::BlockNumber, BalanceOf<T>>>,
        >,
        maybe_insurance_tuple: Option<(
            SideEffectId<T>,
            InsuranceDeposit<T::AccountId, T::BlockNumber, BalanceOf<T>>,
        )>,
    ) -> Result<(), Error<T>> {
        // Apply will try to move the status of Xtx from the current to the closest valid one.
        let current_status = local_ctx.xtx.status.clone();

        match current_status {
            CircuitStatus::Requested => {
                <FullSideEffects<T>>::insert::<
                    XExecSignalId<T>,
                    Vec<Vec<FullSideEffect<T::AccountId, T::BlockNumber, BalanceOf<T>>>>,
                >(local_ctx.xtx_id.clone(), full_ordered_side_effects.clone());

                let mut ids_with_insurance: Vec<SideEffectId<T>> = vec![];
                for (side_effect_id, insurance_deposit) in &local_ctx.insurance_deposits {
                    <InsuranceDeposits<T>>::insert::<
                        XExecSignalId<T>,
                        SideEffectId<T>,
                        InsuranceDeposit<T::AccountId, T::BlockNumber, BalanceOf<T>>,
                    >(
                        local_ctx.xtx_id.clone(),
                        side_effect_id.clone(),
                        insurance_deposit.clone(),
                    );
                    ids_with_insurance.push(*side_effect_id);
                }
                <XtxInsuranceLinks<T>>::insert::<XExecSignalId<T>, Vec<SideEffectId<T>>>(
                    local_ctx.xtx_id.clone(),
                    ids_with_insurance,
                );
                local_ctx.xtx.status = CircuitStatus::determine_xtx_status(
                    full_ordered_side_effects,
                    &local_ctx.insurance_deposits,
                )?;

                <XExecSignals<T>>::insert::<
                    XExecSignalId<T>,
                    XExecSignal<T::AccountId, T::BlockNumber, BalanceOf<T>>,
                >(local_ctx.xtx_id.clone(), local_ctx.xtx.clone());
            }
            CircuitStatus::PendingInsurance => {
                if let Some((side_effect_id, insurance_deposit)) = maybe_insurance_tuple {
                    <Self as Store>::InsuranceDeposits::mutate(
                        local_ctx.xtx_id,
                        side_effect_id,
                        |x| *x = insurance_deposit,
                    );
                } else {
                    return Err(Error::<T>::ApplyFailed);
                }
            }
            _ => unimplemented!(),
        }
        Ok(())
    }

    fn emit(
        xtx_id: XExecSignalId<T>,
        requester: &T::AccountId,
        side_effects: &Vec<SideEffect<T::AccountId, T::BlockNumber, BalanceOf<T>>>,
        _sequential: bool,
    ) {
        Self::deposit_event(Event::XTransactionReceivedForExec(xtx_id.clone()));

        Self::deposit_event(Event::NewSideEffectsAvailable(
            requester.clone(),
            xtx_id.clone(),
            // ToDo: Emit circuit outbound messages -> side effects
            side_effects.to_vec(),
        ));
        // ToDo: Align with ExecDelivery::submit_side_effect
        // <T as pallet_exec_delivery::Config>::submit_side_effect(
        //     xtx_id,
        //     requester: requester.clone(),
        //     side_effects,
        //     sequential,
        // );
    }

    fn charge(requester: &T::AccountId, fee: BalanceOf<T>) -> Result<BalanceOf<T>, Error<T>> {
        let available_trn_balance = <T as EscrowTrait>::Currency::free_balance(requester);
        let new_balance = available_trn_balance.saturating_sub(fee);
        let VAULT: T::AccountId = Default::default();
        <T as EscrowTrait>::Currency::transfer(requester, &VAULT, fee, AllowDeath)
            .map_err(|_| Error::<T>::ChargingTransferFailed)?; // should not fail
        Ok(new_balance)
    }

    fn authorize(
        origin: OriginFor<T>,
        role: CircuitRole,
    ) -> Result<T::AccountId, sp_runtime::traits::BadOrigin> {
        match role {
            CircuitRole::Requester => ensure_signed(origin),
            // ToDo: Handle active Relayer authorisation
            CircuitRole::Relayer => ensure_signed(origin),
            // ToDo: Handle other CircuitRoles
            _ => unimplemented!(),
        }
    }

    fn validate(
        side_effects: &Vec<SideEffect<T::AccountId, T::BlockNumber, BalanceOf<T>>>,
        local_ctx: &mut LocalXtxCtx<T>,
        requester: &T::AccountId,
        sequential: bool,
    ) -> Result<Vec<Vec<FullSideEffect<T::AccountId, T::BlockNumber, BalanceOf<T>>>>, &'static str>
    {
        let mut full_side_effects: Vec<FullSideEffect<T::AccountId, T::BlockNumber, BalanceOf<T>>> =
            vec![];

        for side_effect in side_effects.iter() {
            // ToDo: Generate Circuit's params as default ABI from let abi = pallet_xdns::get_abi(target_id)
            let gateway_abi = Default::default();
            local_ctx.use_protocol.notice_gateway(side_effect.target);
            local_ctx
                .use_protocol
                .validate_args::<T::AccountId, T::BlockNumber, BalanceOf<T>, SystemHashing<T>>(
                    side_effect.clone(),
                    gateway_abi,
                    &mut local_ctx.local_state,
                )?;

            if let Some(insurance_and_reward) =
                UniversalSideEffectsProtocol::check_if_insurance_required::<
                    T::AccountId,
                    T::BlockNumber,
                    BalanceOf<T>,
                    SystemHashing<T>,
                >(side_effect.clone(), &mut local_ctx.local_state)?
            {
                let (insurance, reward) = (insurance_and_reward[0], insurance_and_reward[1]);
                Self::request_side_effect_insurance(
                    &mut local_ctx.insurance_deposits,
                    side_effect.generate_id::<SystemHashing<T>>(),
                    insurance,
                    reward,
                    requester,
                )?;
            }
            full_side_effects.push(FullSideEffect {
                input: side_effect.clone(),
                confirmed: None,
            })
        }

        let full_side_effects_steps: Vec<
            Vec<FullSideEffect<T::AccountId, T::BlockNumber, BalanceOf<T>>>,
        > = match sequential {
            false => vec![full_side_effects],
            true => {
                let mut sequential_order: Vec<
                    Vec<FullSideEffect<T::AccountId, T::BlockNumber, BalanceOf<T>>>,
                > = vec![];
                for fse in full_side_effects.iter() {
                    sequential_order.push(vec![fse.clone()]);
                }
                sequential_order
            }
        };

        Ok(full_side_effects_steps)
    }

    /// On-submit
    fn request_side_effect_insurance(
        insurance_deposits: &mut Vec<(
            SideEffectId<T>,
            InsuranceDeposit<T::AccountId, T::BlockNumber, BalanceOf<T>>,
        )>,
        side_effect_id: SideEffectId<T>,
        insurance: BalanceOf<T>,
        promised_reward: BalanceOf<T>,
        requester: &T::AccountId,
    ) -> Result<(), Error<T>> {
        Self::charge(requester, promised_reward)?;

        insurance_deposits.push((
            side_effect_id,
            InsuranceDeposit::new(
                insurance,
                promised_reward,
                requester.clone(),
                <frame_system::Pallet<T>>::block_number(),
            ),
        ));

        Ok(())
    }

    fn deposit_side_effect_insurance_lock() -> Result<(), &'static str> {
        Ok(())
    }
}
