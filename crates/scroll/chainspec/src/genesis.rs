//! Scroll types for genesis data.

use alloy_primitives::Address;
use alloy_serde::OtherFields;
use serde::de::Error;

/// Container type for all Scroll specific fields in a genesis file.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollChainInfo {
    /// Genesis information
    pub genesis_info: Option<ScrollGenesisInfo>,
    /// scroll chain special information
    pub scroll_special_info: Option<ScrollSpecialChainInfo>,
}

impl ScrollChainInfo {
    /// Extracts the Scroll specific fields from a genesis file. These fields are expected to be
    /// contained in the `genesis.config` under `extra_fields` property.
    pub fn extract_from(others: &OtherFields) -> Option<Self> {
        Self::try_from(others).ok()
    }
}

impl TryFrom<&OtherFields> for ScrollChainInfo {
    type Error = serde_json::Error;

    fn try_from(others: &OtherFields) -> Result<Self, Self::Error> {
        let genesis_info = ScrollGenesisInfo::try_from(others).ok();
        let scroll_special_info = ScrollSpecialChainInfo::try_from(others).ok();

        Ok(Self { genesis_info, scroll_special_info })
    }
}

/// The Scroll-specific genesis block specification.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollGenesisInfo {
    /// bernoulli block number
    pub bernoulli_block: Option<u64>,
    /// curie block number
    pub curie_block: Option<u64>,
    /// darwin hardfork timestamp
    pub darwin_time: Option<u64>,
    /// darwinV2 hardfork timestamp
    pub darwin_v2_time: Option<u64>,
}

impl ScrollGenesisInfo {
    /// Extract the Optimism-specific genesis info from a genesis file.
    pub fn extract_from(others: &OtherFields) -> Option<Self> {
        Self::try_from(others).ok()
    }
}

impl TryFrom<&OtherFields> for ScrollGenesisInfo {
    type Error = serde_json::Error;

    fn try_from(others: &OtherFields) -> Result<Self, Self::Error> {
        others.deserialize_as()
    }
}

/// The Scroll l1 special config
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollL1Config {
    /// l1 chain id
    pub l1_chainId: Option<u64>,
    /// the l1 message queue address
    pub l1_message_queue_address: Option<Address>,
    // the l1 scroll proxy address
    pub l1_chain_proxy_address: Option<Address>,
    // the l1 message numbers of per block
    pub num_l1_messages_per_block: Option<u64>,
}

/// The Scroll special chain specification.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollSpecialChainInfo {
    /// the L2 tx fee vault address
    pub fee_vault_address: Option<Address>,
    /// the L1 special config
    pub scroll_l1_config: Option<ScrollL1Config>,
}

impl ScrollSpecialChainInfo {
    /// Extracts the scroll special info by looking for the `scroll` key. It is intended to be
    /// parsed from a genesis file.
    pub fn extract_from(others: &OtherFields) -> Option<Self> {
        Self::try_from(others).ok()
    }
}

impl TryFrom<&OtherFields> for ScrollSpecialChainInfo {
    type Error = serde_json::Error;

    fn try_from(others: &OtherFields) -> Result<Self, Self::Error> {
        if let Some(Ok(scroll_chain_special_info)) = others.get_deserialized::<Self>("scroll") {
            Ok(scroll_chain_special_info)
        } else {
            Err(serde_json::Error::missing_field("scroll"))
        }
    }
}
