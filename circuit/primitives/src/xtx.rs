use crate::side_effect::*;
use codec::{Decode, Encode};
use sp_runtime::{
    traits::{Hash, Zero},
    RuntimeDebug,
};
use sp_std::vec::Vec;
type SystemHashing<T> = <T as frame_system::Config>::Hashing;
pub type XtxId<T> = <T as frame_system::Config>::Hash;
use sp_std::fmt::Debug;

/// A composable cross-chain (X) transaction that has already been verified to be valid and submittable
#[derive(Clone, Eq, PartialEq, Default, Encode, Decode, RuntimeDebug)]
pub struct Xtx<AccountId, BlockNumber, BalanceOf> {
    // todo: Add missing DFDs
    // pub contracts_dfd: InterExecSchedule -> ContractsDFD
    // pub side_effects_dfd: SideEffectsDFD
    // pub gateways_dfd: GatewaysDFD
    /// The owner of the bid
    pub requester: AccountId,

    /// Encoded content of composable tx
    pub initial_input: Vec<u8>,

    /// Expiry timeout
    pub timeouts_at: Option<BlockNumber>,

    /// Schedule execution of steps in the future intervals
    pub delay_steps_at: Option<Vec<BlockNumber>>,

    /// Has returned status already and what
    pub result_status: Option<Vec<u8>>,

    /// Total reward
    pub total_reward: Option<BalanceOf>,

    /// Vector of Steps that each can consist out of at least one FullSideEffect
    pub full_side_effects: Vec<Vec<FullSideEffect<AccountId, BlockNumber, BalanceOf>>>,
}

impl<
        AccountId: Encode + Clone + Debug,
        BlockNumber: Ord + Copy + Zero + Encode + Clone + Debug,
        BalanceOf: Copy + Zero + Encode + Decode + Clone + Debug,
    > Xtx<AccountId, BlockNumber, BalanceOf>
{
    pub fn new(
        // Requester of xtx
        requester: AccountId,
        // Encoded initial input set by a requester/SDK - base for the xtx state
        initial_input: Vec<u8>,
        // Expiry timeout
        timeouts_at: Option<BlockNumber>,
        // Schedule execution of steps in the future intervals
        delay_steps_at: Option<Vec<BlockNumber>>,
        // Total reward
        total_reward: Option<BalanceOf>,

        full_side_effects: Vec<Vec<FullSideEffect<AccountId, BlockNumber, BalanceOf>>>,
    ) -> Self {
        Xtx {
            requester,
            initial_input,
            timeouts_at,
            delay_steps_at,
            result_status: None,
            total_reward,
            full_side_effects,
        }
    }

    pub fn generate_xtx_id<T: frame_system::Config>(&self) -> XtxId<T> {
        SystemHashing::<T>::hash(Encode::encode(self).as_ref())
    }

    // Complete the full side effect of Xtx by assigning confirmed side effect/
    // This can only happen if the side effects is confirmed with respect to
    // the execution steps.
    //
    // Return true if the input side effect successfully confirmed.
    // Return false if no update has happened.
    // Throw an error if detected the confirmation ruleset has been violated.
    pub fn complete_side_effect<Hasher: sp_core::Hasher>(
        &mut self,
        confirmed: ConfirmedSideEffect<AccountId, BlockNumber, BalanceOf>,
        input: SideEffect<AccountId, BlockNumber, BalanceOf>,
    ) -> Result<bool, &'static str> {
        let input_side_effect_id = input.generate_id::<Hasher>();

        // Double check there are some side effects for that Xtx - should have been checked at API level tho already
        if self.full_side_effects.is_empty() {
            return Err("Xtx has no single side effect step to confirm");
        }

        let mut step_with_unconfirmed_steps: Option<usize> = None;

        for (i, step) in self.full_side_effects.iter_mut().enumerate() {
            // Double check there are some side effects for that Xtx - should have been checked at API level tho already
            if step.is_empty() {
                return Err("Xtx has an empty single step.");
            }
            for mut full_side_effect in step.iter_mut() {
                if full_side_effect.confirmed.is_none() {
                    step_with_unconfirmed_steps = Some(i);
                    // Recalculate the ID for each input side effect and compare with the input one.
                    // Check the current unconfirmed step before attempt to confirm the full side effect.
                    if full_side_effect.input.generate_id::<Hasher>() == input_side_effect_id
                        && step_with_unconfirmed_steps == Some(i)
                    {
                        // We found the side effect to confirm from inside the unconfirmed step.
                        full_side_effect.confirmed = Some(confirmed.clone());
                        return Ok(true);
                    } else {
                        return Err("Attempt to confirm side effect from the next step, \
                                but there still is at least one unfishised step");
                    }
                }
            }
        }

        return Ok(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type BlockNumber = u64;
    type BalanceOf = u64;
    type AccountId = u64;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type Hash = sp_core::H256;

    #[test]
    fn successfully_creates_empty_xtx() {
        let empty_xtx =
            Xtx::<AccountId, BlockNumber, BalanceOf>::new(0, vec![], None, None, None, vec![]);

        assert_eq!(
            empty_xtx,
            Xtx {
                requester: 0,
                initial_input: vec![],
                timeouts_at: None,
                delay_steps_at: None,
                result_status: None,
                total_reward: None,
                full_side_effects: vec![]
            }
        );
    }

    #[test]
    fn successfully_confirms_1_side_effect_and_completes_xtx() {
        let input_side_effect_1 = SideEffect::<AccountId, BlockNumber, BalanceOf> {
            target: [0, 0, 0, 0],
            prize: 0,
            ordered_at: 0,
            encoded_action: vec![],
            encoded_args: vec![],
            signature: vec![],
            enforce_executioner: None,
        };

        let completing_side_effect_1 = ConfirmedSideEffect::<AccountId, BlockNumber, BalanceOf> {
            err: None,
            output: None,
            inclusion_proof: None,
            executioner: 1,
            received_at: 1,
            cost: None,
        };

        let mut xtx = Xtx::<AccountId, BlockNumber, BalanceOf>::new(
            0,
            vec![],
            None,
            None,
            None,
            vec![vec![FullSideEffect {
                input: input_side_effect_1.clone(),
                confirmed: None,
            }]],
        );

        let res = xtx
            .complete_side_effect::<Hashing>(
                completing_side_effect_1.clone(),
                input_side_effect_1.clone(),
            )
            .unwrap();

        assert_eq!(res, true);
        // Check that xtx.full_side_effects has been updated
        assert_eq!(
            xtx.full_side_effects[0][0],
            FullSideEffect {
                input: input_side_effect_1,
                confirmed: Some(completing_side_effect_1),
            }
        );
    }

    #[test]
    fn successfully_confirms_2_side_effect_in_1_step_in_xtx() {
        let input_side_effect_1 = SideEffect::<AccountId, BlockNumber, BalanceOf> {
            target: [0, 0, 0, 0],
            prize: 0,
            ordered_at: 0,
            encoded_action: vec![],
            encoded_args: vec![],
            signature: vec![],
            enforce_executioner: None,
        };

        let input_side_effect_2 = SideEffect::<AccountId, BlockNumber, BalanceOf> {
            target: [0, 0, 0, 1],
            prize: 0,
            ordered_at: 0,
            encoded_action: vec![],
            encoded_args: vec![],
            signature: vec![],
            enforce_executioner: None,
        };

        let completing_side_effect_1 = ConfirmedSideEffect::<AccountId, BlockNumber, BalanceOf> {
            err: None,
            output: None,
            inclusion_proof: None,
            executioner: 1,
            received_at: 1,
            cost: None,
        };

        let completing_side_effect_2 = ConfirmedSideEffect::<AccountId, BlockNumber, BalanceOf> {
            err: None,
            output: None,
            inclusion_proof: None,
            executioner: 2,
            received_at: 1,
            cost: None,
        };

        let mut xtx = Xtx::<AccountId, BlockNumber, BalanceOf>::new(
            0,
            vec![],
            None,
            None,
            None,
            vec![vec![
                FullSideEffect {
                    input: input_side_effect_1.clone(),
                    confirmed: None,
                },
                FullSideEffect {
                    input: input_side_effect_2.clone(),
                    confirmed: None,
                },
            ]],
        );

        let res_1 = xtx
            .complete_side_effect::<Hashing>(
                completing_side_effect_1.clone(),
                input_side_effect_1.clone(),
            )
            .unwrap();

        assert_eq!(res_1, true);
        // Check that the first xtx.full_side_effects has been updated
        assert_eq!(
            xtx.full_side_effects[0][0],
            FullSideEffect {
                input: input_side_effect_1.clone(),
                confirmed: Some(completing_side_effect_1),
            }
        );

        // Check that the second xtx.full_side_effects has NOT been updated
        assert_eq!(
            xtx.full_side_effects[0][1],
            FullSideEffect {
                input: input_side_effect_2.clone(),
                confirmed: None,
            }
        );

        let res_2 = xtx
            .complete_side_effect::<Hashing>(
                completing_side_effect_2.clone(),
                input_side_effect_2.clone(),
            )
            .unwrap();

        assert_eq!(res_2, true);

        // Check that the second xtx.full_side_effects has now been updated
        assert_eq!(
            xtx.full_side_effects[0][1],
            FullSideEffect {
                input: input_side_effect_2,
                confirmed: Some(completing_side_effect_2),
            }
        );
    }

    // #[test]
    // fn successfully_confirms_2_side_effect_in_2_steps_in_xtx() {
    //     todo!()
    // }
    //
    // #[test]
    // fn throws_when_attempts_to_confirm_side_effect_from_2nd_step_without_1st_in_xtx() {
    //     todo!()
    // }
}
