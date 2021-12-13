#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::boxed::Box;
use sp_std::collections::btree_map::BTreeMap;

use sp_std::vec::*;

use t3rn_primitives::abi::GatewayABIConfig;
use t3rn_primitives::side_effect::{SideEffect, TargetId};

use crate::side_effects::protocol::SideEffectProtocol;
use crate::side_effects::protocol::SideEffectProtocol as SideEffectProtocolT;
use crate::side_effects::protocol::{CallSideEffectProtocol, TransferSideEffectProtocol};
use t3rn_primitives::volatile::LocalState;

pub type EventSignature = Vec<u8>;
pub type String = Vec<u8>;

pub trait SideEffectsLazyLoader {
    fn notice_gateway(&mut self, gateway_id: TargetId);
    fn select_side_effect<
        AccountId,
        BlockNumber,
        BalanceOf,
        CB: FnOnce(&Box<dyn SideEffectProtocolT>) -> Result<(), &'static str>,
    >(
        &self,
        side_effect: SideEffect<AccountId, BlockNumber, BalanceOf>,
        matched_cb: CB,
    ) -> Result<(), &'static str>;
}

// F: FnOnce(&[(Vec<u8>, &[u8])]) -> R,
impl SideEffectsLazyLoader for UniversalSideEffectsProtocol {
    fn notice_gateway(&mut self, gateway_id: TargetId) {
        // ToDo: Just load the std side effects for the gateway for now
        //    but missing implementation would:
        //     1. compare the "allowed_methods" with std side effects and load only selected ones
        //     2. load from XDNS memory / or receive already pre-fetched custom side effects for that gateway
        if !self.seen_side_effects_protocol.contains_key(&gateway_id) {
            // Load standard side effects protocol
            let mut known_std_side_effects: BTreeMap<&'static str, Box<dyn SideEffectProtocolT>> =
                BTreeMap::new();
            let transfer = TransferSideEffectProtocol {};
            let call = CallSideEffectProtocol {};
            known_std_side_effects.insert(transfer.get_name(), Box::new(transfer.clone()));
            known_std_side_effects.insert(call.get_name(), Box::new(call.clone()));
            self.seen_side_effects_protocol
                .insert(gateway_id, known_std_side_effects);
        }
    }

    fn select_side_effect<
        AccountId,
        BlockNumber,
        BalanceOf,
        CB: FnOnce(&Box<dyn SideEffectProtocolT>) -> Result<(), &'static str>,
    >(
        &self,
        side_effect: SideEffect<AccountId, BlockNumber, BalanceOf>,
        matched_cb: CB,
    ) -> Result<(), &'static str> {
        match self.seen_side_effects_protocol.get(&side_effect.target) {
            Some(available_side_effects) => {
                if side_effect.encoded_action == b"transfer".to_vec() {
                    match available_side_effects.get("transfer:dirty") {
                        Some(transfer_side_effect_protocol) => {
                            matched_cb(transfer_side_effect_protocol)
                        }
                        _ => Err("UniversalSideEffectsProtocol::validate_args - side effect unsupported on chosen gateway"),
                    }
                } else {
                    Err("UniversalSideEffectsProtocol::validate_args - unknown side effect type")
                }
            }
            _ => Err("UniversalSideEffectsProtocol::validate_args - unknown gateway"),
        }
    }
}

pub struct UniversalSideEffectsProtocol {
    pub seen_side_effects_protocol:
        BTreeMap<TargetId, BTreeMap<&'static str, Box<dyn SideEffectProtocolT>>>,
}

impl UniversalSideEffectsProtocol {
    pub fn new() -> Self {
        Self {
            seen_side_effects_protocol: BTreeMap::new(),
        }
    }

    pub fn validate_args<AccountId: Clone, BlockNumber: Clone, BalanceOf: Clone>(
        &self,
        side_effect: SideEffect<AccountId, BlockNumber, BalanceOf>,
        gateway_abi: GatewayABIConfig,
        local_state: &mut LocalState,
    ) -> Result<(), &'static str> {
        self.select_side_effect(
            side_effect.clone(),
            |side_effect_protocol: &Box<dyn SideEffectProtocolT>| {
                side_effect_protocol.validate_args(
                    side_effect.encoded_args,
                    gateway_abi,
                    local_state,
                )
            },
        )
    }
}
