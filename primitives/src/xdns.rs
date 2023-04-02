use crate::{
    gateway::GatewayABIConfig, ChainId, GatewayGenesisConfig, GatewayType, GatewayVendor,
    TokenSysProps,
};
use codec::{Decode, Encode};
use frame_support::dispatch::{DispatchResult, DispatchResultWithPostInfo};
use frame_system::pallet_prelude::OriginFor;
use scale_info::TypeInfo;
use sp_runtime::DispatchError;
use sp_std::vec::Vec;
use t3rn_abi::sfx_abi::SFXAbi;
use t3rn_types::{fsx::SecurityLvl, sfx::Sfx4bId};

/// A hash based on encoding the complete XdnsRecord
pub type XdnsRecordId = [u8; 4];

/// A hash based on encoding the Gateway ID
pub type XdnsGatewayId<T> = <T as frame_system::Config>::Hash;

#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Parachain {
    // gateway_id of relaychain
    pub relay_chain_id: ChainId,
    // parachain_id
    pub id: u32,
}

/// A preliminary representation of a xdns_record in the onchain registry.
#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct TokenRecord {
    /// Its token 4b Id
    pub token_id: [u8; 4],

    /// Link to the gateway token is whitelisted on.
    pub gateway_id: [u8; 4],

    /// Token properties - decimals, symbol, name
    pub token_props: TokenSysProps,
}

/// A preliminary representation of a xdns_record in the onchain registry.
#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct GatewayRecord<AccountId> {
    /// Gateway 4b Id
    pub gateway_id: ChainId,

    /// Verification Vendor / Light Client or internal (XCM/XBI)
    pub verification_vendor: GatewayVendor,

    /// Default encoding for the gateway
    pub codec: t3rn_abi::Codec,

    /// Optional owner
    pub registrant: Option<AccountId>,

    /// Leave empty if there's no escrow capabilities on the remote gateway
    pub escrow_account: Option<AccountId>,

    /// Methods enabled to be called on the remote target: (Sfx4bId, Option<PalletIndexMemo>)
    pub allowed_side_effects: Vec<(Sfx4bId, Option<u8>)>,
}

/// A preliminary representation of a xdns_record in the onchain registry.
#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct XdnsRecord<AccountId> {
    /// SCALE-encoded url string on where given Consensus System can be accessed
    pub url: Vec<u8>,

    pub gateway_abi: GatewayABIConfig,

    pub gateway_genesis: GatewayGenesisConfig,

    /// Gateway Vendor
    pub gateway_vendor: GatewayVendor,

    /// Gateway Type
    pub gateway_type: GatewayType,

    /// Gateway Id
    pub gateway_id: ChainId,

    pub parachain: Option<Parachain>,

    /// Gateway System Properties
    pub gateway_sys_props: TokenSysProps,

    pub registrant: Option<AccountId>,

    /// Leave empty if there's no escrow capabilities on the remote gateway
    pub security_coordinates: Vec<u8>,

    pub last_finalized: Option<u64>,

    /// Methods enabled to be called on the remote target
    pub allowed_side_effects: Vec<Sfx4bId>,
}

impl<AccountId: Encode> XdnsRecord<AccountId> {
    pub fn new_from_primitives(
        url: Vec<u8>,
        gateway_abi: GatewayABIConfig,
        modules_encoded: Option<Vec<u8>>,
        extrinsics_version: u8,
        genesis_hash: Vec<u8>,
        gateway_id: ChainId,
        parachain: Option<Parachain>,
        gateway_vendor: GatewayVendor,
        gateway_type: GatewayType,
        gateway_sys_props: TokenSysProps,
        registrant: Option<AccountId>,
        security_coordinates: Vec<u8>,
        last_finalized: Option<u64>,
        allowed_side_effects: Vec<Sfx4bId>,
    ) -> Self {
        let gateway_genesis = GatewayGenesisConfig {
            modules_encoded,
            extrinsics_version,
            genesis_hash,
        };

        XdnsRecord {
            url,
            gateway_abi,
            gateway_genesis,
            gateway_vendor,
            gateway_type,
            gateway_id,
            parachain,
            gateway_sys_props,
            registrant,
            security_coordinates,
            last_finalized,
            allowed_side_effects,
        }
    }

    pub fn new(
        url: Vec<u8>,
        gateway_id: ChainId,
        parachain: Option<Parachain>,
        gateway_abi: GatewayABIConfig,
        gateway_vendor: GatewayVendor,
        gateway_type: GatewayType,
        gateway_genesis: GatewayGenesisConfig,
        gateway_sys_props: TokenSysProps,
        security_coordinates: Vec<u8>,
        allowed_side_effects: Vec<Sfx4bId>,
    ) -> Self {
        XdnsRecord {
            url,
            gateway_id,
            parachain,
            gateway_abi,
            gateway_vendor,
            gateway_type,
            gateway_genesis,
            gateway_sys_props,
            registrant: None,
            security_coordinates,
            last_finalized: None,
            allowed_side_effects,
        }
    }

    pub fn assign_registrant(&mut self, registrant: AccountId) {
        self.registrant = Some(registrant)
    }

    /// Function that generates an XdnsRecordId hash based on the gateway id
    pub fn generate_id<T: frame_system::Config>(&self) -> XdnsRecordId {
        self.gateway_id
    }

    pub fn set_last_finalized(&mut self, last_finalized: u64) {
        self.last_finalized = Some(last_finalized)
    }
}

pub trait Xdns<T: frame_system::Config> {
    /// Fetches all known XDNS records
    fn fetch_records() -> Vec<XdnsRecord<T::AccountId>>;

    fn fetch_gateways() -> Vec<GatewayRecord<T::AccountId>>;

    fn add_new_token(
        token_id: [u8; 4],
        gateway_id: [u8; 4],
        token_props: TokenSysProps,
    ) -> DispatchResult;

    fn override_token(
        token_id: [u8; 4],
        gateway_id: [u8; 4],
        token_props: TokenSysProps,
    ) -> DispatchResult;

    fn add_new_gateway(
        gateway_id: [u8; 4],
        verification_vendor: GatewayVendor,
        codec: t3rn_abi::Codec,
        registrant: Option<T::AccountId>,
        escrow_account: Option<T::AccountId>,
        allowed_side_effects: Vec<([u8; 4], Option<u8>)>,
    ) -> DispatchResult;

    fn override_gateway(
        gateway_id: [u8; 4],
        verification_vendor: GatewayVendor,
        codec: t3rn_abi::Codec,
        registrant: Option<T::AccountId>,
        escrow_account: Option<T::AccountId>,
        allowed_side_effects: Vec<([u8; 4], Option<u8>)>,
    ) -> DispatchResult;

    fn extend_sfx_abi(
        origin: OriginFor<T>,
        gateway_id: ChainId,
        sfx_4b_id: Sfx4bId,
        sfx_expected_abi: SFXAbi,
    ) -> DispatchResult;

    fn override_sfx_abi(
        origin: OriginFor<T>,
        gateway_id: ChainId,
        new_sfx_abi: Vec<(Sfx4bId, SFXAbi)>,
    ) -> DispatchResult;

    fn get_all_sfx_abi(gateway_id: &ChainId) -> Vec<(Sfx4bId, SFXAbi)>;

    fn get_sfx_abi(gateway_id: &ChainId, sfx_4b_id: Sfx4bId) -> Option<SFXAbi>;

    fn add_escrow_account(
        origin: OriginFor<T>,
        gateway_id: ChainId,
        escrow_account: T::AccountId,
    ) -> DispatchResult;

    fn allowed_side_effects(gateway_id: &ChainId) -> Vec<([u8; 4], Option<u8>)>;

    fn get_gateway_type_unsafe(chain_id: &ChainId) -> GatewayType;

    fn get_verification_vendor(chain_id: &ChainId) -> Result<GatewayVendor, DispatchError>;

    fn get_escrow_account(chain_id: &ChainId) -> Result<Vec<u8>, DispatchError>;
}
