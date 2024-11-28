//! Hard forks of scroll protocol.

use core::{
    any::Any,
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use alloy_chains::Chain;
use reth_ethereum_forks::{hardfork, ChainHardforks, EthereumHardfork, ForkCondition, Hardfork};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

hardfork!(
    /// The name of the Scroll hardfork
    ///
    /// When building a list of hardforks for a chain, it's still expected to mix with
    /// [`EthereumHardfork`].
    ScrollHardFork {
        /// Bernoulli: <https://scroll.io/blog/blobs-are-here-scrolls-bernoulli-upgrade>.
        Bernoulli,
        /// Curie: <https://scroll.io/blog/compressing-the-gas-scrolls-curie-upgrade>.
        Curie,
        /// Darwin: <https://scroll.io/blog/proof-recursion-scrolls-darwin-upgrade>.
        Darwin,
        /// DarwinV2 <https://x.com/Scroll_ZKP/status/1830565514755584269>.
        DarwinV2,
    }
);

impl ScrollHardFork {
    /// Retrieves the activation block for the specified hardfork on the given chain.
    pub fn activation_block<H: Hardfork>(self, fork: H, chain: Chain) -> Option<u64> {
        if chain == Chain::base_sepolia() {
            return Self::base_sepolia_activation_block(fork);
        }
        if chain == Chain::base_mainnet() {
            return Self::base_mainnet_activation_block(fork);
        }

        None
    }

    /// Retrieves the activation timestamp for the specified hardfork on the given chain.
    pub fn activation_timestamp<H: Hardfork>(self, fork: H, chain: Chain) -> Option<u64> {
        if chain == Chain::base_sepolia() {
            return Self::base_sepolia_activation_timestamp(fork);
        }
        if chain == Chain::base_mainnet() {
            return Self::base_mainnet_activation_timestamp(fork);
        }

        None
    }

    /// Retrieves the activation block for the specified hardfork on the Base Sepolia testnet.
    pub fn base_sepolia_activation_block<H: Hardfork>(fork: H) -> Option<u64> {
        match_hardfork(
            fork,
            |fork| match fork {
                EthereumHardfork::Frontier
                | EthereumHardfork::Homestead
                | EthereumHardfork::Dao
                | EthereumHardfork::Tangerine
                | EthereumHardfork::SpuriousDragon
                | EthereumHardfork::Byzantium
                | EthereumHardfork::Constantinople
                | EthereumHardfork::Petersburg
                | EthereumHardfork::Istanbul
                | EthereumHardfork::MuirGlacier
                | EthereumHardfork::Berlin
                | EthereumHardfork::London
                | EthereumHardfork::ArrowGlacier
                | EthereumHardfork::GrayGlacier
                | EthereumHardfork::Paris
                | EthereumHardfork::Shanghai
                | EthereumHardfork::Cancun => Some(0),
                _ => None,
            },
            |fork| match fork {
                Self::Bernoulli => Some(3747132),
                Self::Curie => Some(4740239),
                Self::Darwin => Some(6075509),
                Self::DarwinV2 => Some(6375501),
                _ => None,
            },
        )
    }

    /// Retrieves the activation block for the specified hardfork on the Base mainnet.
    pub fn base_mainnet_activation_block<H: Hardfork>(fork: H) -> Option<u64> {
        match_hardfork(
            fork,
            |fork| match fork {
                EthereumHardfork::Frontier
                | EthereumHardfork::Homestead
                | EthereumHardfork::Dao
                | EthereumHardfork::Tangerine
                | EthereumHardfork::SpuriousDragon
                | EthereumHardfork::Byzantium
                | EthereumHardfork::Constantinople
                | EthereumHardfork::Petersburg
                | EthereumHardfork::Istanbul
                | EthereumHardfork::MuirGlacier
                | EthereumHardfork::Berlin
                | EthereumHardfork::London
                | EthereumHardfork::ArrowGlacier
                | EthereumHardfork::GrayGlacier
                | EthereumHardfork::Paris
                | EthereumHardfork::Shanghai
                | EthereumHardfork::Cancun => Some(0),
                _ => None,
            },
            |fork| match fork {
                Self::Bernoulli => Some(5220340),
                Self::Curie => Some(7096836),
                Self::Darwin => Some(8568134),
                Self::DarwinV2 => Some(8923772),
                _ => None,
            },
        )
    }

    /// Retrieves the activation timestamp for the specified hardfork on the Base Sepolia testnet.
    pub fn base_sepolia_activation_timestamp<H: Hardfork>(fork: H) -> Option<u64> {
        match_hardfork(
            fork,
            |fork| match fork {
                EthereumHardfork::Frontier
                | EthereumHardfork::Homestead
                | EthereumHardfork::Dao
                | EthereumHardfork::Tangerine
                | EthereumHardfork::SpuriousDragon
                | EthereumHardfork::Byzantium
                | EthereumHardfork::Constantinople
                | EthereumHardfork::Petersburg
                | EthereumHardfork::Istanbul
                | EthereumHardfork::MuirGlacier
                | EthereumHardfork::Berlin
                | EthereumHardfork::London
                | EthereumHardfork::ArrowGlacier
                | EthereumHardfork::GrayGlacier
                | EthereumHardfork::Paris
                | EthereumHardfork::Shanghai
                | EthereumHardfork::Cancun => Some(0),
                _ => None,
            },
            |fork| match fork {
                Self::Bernoulli => Some(1713175866),
                Self::Curie => Some(1718616171),
                Self::Darwin => Some(1723622400),
                Self::DarwinV2 => Some(1724832000),
                _ => None,
            },
        )
    }

    /// Retrieves the activation timestamp for the specified hardfork on the Base mainnet.
    pub fn base_mainnet_activation_timestamp<H: Hardfork>(fork: H) -> Option<u64> {
        match_hardfork(
            fork,
            |fork| match fork {
                EthereumHardfork::Frontier
                | EthereumHardfork::Homestead
                | EthereumHardfork::Dao
                | EthereumHardfork::Tangerine
                | EthereumHardfork::SpuriousDragon
                | EthereumHardfork::Byzantium
                | EthereumHardfork::Constantinople
                | EthereumHardfork::Petersburg
                | EthereumHardfork::Istanbul
                | EthereumHardfork::MuirGlacier
                | EthereumHardfork::Berlin
                | EthereumHardfork::London
                | EthereumHardfork::ArrowGlacier
                | EthereumHardfork::GrayGlacier
                | EthereumHardfork::Paris
                | EthereumHardfork::Shanghai
                | EthereumHardfork::Cancun => Some(0),
                _ => None,
            },
            |fork| match fork {
                Self::Bernoulli => Some(1714358352),
                Self::Curie => Some(1719994277),
                Self::Darwin => Some(1724227200),
                Self::DarwinV2 => Some(1725264000),
                _ => None,
            },
        )
    }

    /// Scroll mainnet list of hardforks.
    pub fn scroll_mainnet() -> ChainHardforks {
        ChainHardforks::new(vec![
            (EthereumHardfork::Frontier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Homestead.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Tangerine.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::SpuriousDragon.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Byzantium.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Constantinople.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Petersburg.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Istanbul.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::MuirGlacier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Berlin.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::London.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::ArrowGlacier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::GrayGlacier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Paris.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Shanghai.boxed(), ForkCondition::Timestamp(0)),
            (EthereumHardfork::Cancun.boxed(), ForkCondition::Timestamp(0)),
            (Self::Bernoulli.boxed(), ForkCondition::Block(5220340)),
            (Self::Curie.boxed(), ForkCondition::Block(7096836)),
            (Self::Darwin.boxed(), ForkCondition::Timestamp(1724227200)),
            (Self::DarwinV2.boxed(), ForkCondition::Timestamp(1725264000)),
        ])
    }

    /// Scroll sepolia list of hardforks.
    pub fn scroll_sepolia() -> ChainHardforks {
        ChainHardforks::new(vec![
            (EthereumHardfork::Frontier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Homestead.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Tangerine.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::SpuriousDragon.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Byzantium.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Constantinople.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Petersburg.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Istanbul.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::MuirGlacier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Berlin.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::London.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::ArrowGlacier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::GrayGlacier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Paris.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Shanghai.boxed(), ForkCondition::Timestamp(0)),
            (EthereumHardfork::Cancun.boxed(), ForkCondition::Timestamp(0)),
            (Self::Bernoulli.boxed(), ForkCondition::Block(3747132)),
            (Self::Curie.boxed(), ForkCondition::Block(4740239)),
            (Self::Darwin.boxed(), ForkCondition::Timestamp(1723622400)),
            (Self::DarwinV2.boxed(), ForkCondition::Timestamp(1724832000)),
        ])
    }

    /// Base sepolia list of hardforks.
    pub fn base_sepolia() -> ChainHardforks {
        ChainHardforks::new(vec![
            (EthereumHardfork::Frontier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Homestead.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Tangerine.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::SpuriousDragon.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Byzantium.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Constantinople.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Petersburg.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Istanbul.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::MuirGlacier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Berlin.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::London.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::ArrowGlacier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::GrayGlacier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Paris.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Shanghai.boxed(), ForkCondition::Timestamp(0)),
            (EthereumHardfork::Cancun.boxed(), ForkCondition::Timestamp(0)),
            (Self::Bernoulli.boxed(), ForkCondition::Block(3747132)),
            (Self::Curie.boxed(), ForkCondition::Block(4740239)),
            (Self::Darwin.boxed(), ForkCondition::Timestamp(1723622400)),
            (Self::DarwinV2.boxed(), ForkCondition::Timestamp(1724832000)),
        ])
    }

    /// Base mainnet list of hardforks.
    pub fn base_mainnet() -> ChainHardforks {
        ChainHardforks::new(vec![
            (EthereumHardfork::Frontier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Homestead.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Tangerine.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::SpuriousDragon.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Byzantium.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Constantinople.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Petersburg.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Istanbul.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::MuirGlacier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Berlin.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::London.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::ArrowGlacier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::GrayGlacier.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Paris.boxed(), ForkCondition::Block(0)),
            (EthereumHardfork::Shanghai.boxed(), ForkCondition::Timestamp(1704992401)),
            (EthereumHardfork::Cancun.boxed(), ForkCondition::Timestamp(1710374401)),
            (Self::Bernoulli.boxed(), ForkCondition::Block(5220340)),
            (Self::Curie.boxed(), ForkCondition::Block(7096836)),
            (Self::Darwin.boxed(), ForkCondition::Timestamp(1724227200)),
            (Self::DarwinV2.boxed(), ForkCondition::Timestamp(1725264000)),
        ])
    }
}

/// Match helper method since it's not possible to match on `dyn Hardfork`
fn match_hardfork<H, HF, SHF>(fork: H, hardfork_fn: HF, scroll_hardfork_fn: SHF) -> Option<u64>
where
    H: Hardfork,
    HF: Fn(&EthereumHardfork) -> Option<u64>,
    SHF: Fn(&ScrollHardFork) -> Option<u64>,
{
    let fork: &dyn Any = &fork;
    if let Some(fork) = fork.downcast_ref::<EthereumHardfork>() {
        return hardfork_fn(fork);
    }
    fork.downcast_ref::<ScrollHardFork>().and_then(scroll_hardfork_fn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_hardfork() {
        assert_eq!(
            ScrollHardFork::base_mainnet_activation_block(ScrollHardFork::Bernoulli),
            Some(5220340)
        );
        assert_eq!(
            ScrollHardFork::base_mainnet_activation_block(ScrollHardFork::Curie),
            Some(7096836)
        );
    }

    #[test]
    fn check_scroll_hardfork_from_str() {
        let hardfork_str = ["BernOulLi", "CrUie", "DaRwIn", "DaRwInV2"];
        let expected_hardforks = [
            ScrollHardFork::Bernoulli,
            ScrollHardFork::Curie,
            ScrollHardFork::Darwin,
            ScrollHardFork::DarwinV2,
        ];

        let hardforks: Vec<ScrollHardFork> =
            hardfork_str.iter().map(|h| ScrollHardFork::from_str(h).unwrap()).collect();

        assert_eq!(hardforks, expected_hardforks);
    }

    #[test]
    fn check_nonexistent_hardfork_from_str() {
        assert!(ScrollHardFork::from_str("not a hardfork").is_err());
    }
}
