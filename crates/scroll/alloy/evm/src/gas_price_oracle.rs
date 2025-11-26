//! L1 Gas Price Oracle constants.

use revm::primitives::{address, Address, U256};

/// L1 gas price oracle address.
/// <https://scrollscan.com/address/0x5300000000000000000000000000000000000002>
pub const L1_GAS_PRICE_ORACLE_ADDRESS: Address =
    address!("5300000000000000000000000000000000000002");

// forge inspect src/L2/predeploys/L1GasPriceOracle.sol:L1GasPriceOracle storageLayout
// ╭------------------+---------------------+------+--------+-------╮
// | Name             | Type                | Slot | Offset | Bytes |
// +================================================================+
// | owner            | address             | 0    | 0      | 20    |
// |------------------+---------------------+------+--------+-------+
// | l1BaseFee        | uint256             | 1    | 0      | 32    |
// |------------------+---------------------+------+--------+-------+
// | overhead         | uint256             | 2    | 0      | 32    |
// |------------------+---------------------+------+--------+-------+
// | scalar           | uint256             | 3    | 0      | 32    |
// |------------------+---------------------+------+--------+-------+
// | whitelist        | contract IWhitelist | 4    | 0      | 20    |
// |------------------+---------------------+------+--------+-------+
// | l1BlobBaseFee    | uint256             | 5    | 0      | 32    |
// |------------------+---------------------+------+--------+-------+
// | commitScalar     | uint256             | 6    | 0      | 32    |
// |------------------+---------------------+------+--------+-------+
// | blobScalar       | uint256             | 7    | 0      | 32    |
// |------------------+---------------------+------+--------+-------+
// | isCurie          | bool                | 8    | 0      | 1     |
// |------------------+---------------------+------+--------+-------+
// | penaltyThreshold | uint256             | 9    | 0      | 32    |
// |------------------+---------------------+------+--------+-------+
// | penaltyFactor    | uint256             | 10   | 0      | 32    |
// |------------------+---------------------+------+--------+-------+
// | isFeynman        | bool                | 11   | 0      | 1     |
// |------------------+---------------------+------+--------+-------+
// | __gap            | uint248             | 11   | 1      | 31    |
// |------------------+---------------------+------+--------+-------+
// | isGalileo        | bool                | 12   | 0      | 1     |
// ╰------------------+---------------------+------+--------+-------╯

/// Storage slot for `owner` in the `L1GasPriceOracle` contract.
pub const GPO_OWNER_SLOT: U256 = U256::from_limbs([0, 0, 0, 0]);

/// Storage slot for `l1BaseFee` in the `L1GasPriceOracle` contract.
pub const GPO_L1_BASE_FEE_SLOT: U256 = U256::from_limbs([1, 0, 0, 0]);

/// Storage slot for `overhead` in the `L1GasPriceOracle` contract.
pub const GPO_OVERHEAD_SLOT: U256 = U256::from_limbs([2, 0, 0, 0]);

/// Storage slot for `scalar` in the `L1GasPriceOracle` contract.
pub const GPO_SCALAR_SLOT: U256 = U256::from_limbs([3, 0, 0, 0]);

/// Storage slot for `whitelist` in the `L1GasPriceOracle` contract.
pub const GPO_WHITELIST_SLOT: U256 = U256::from_limbs([4, 0, 0, 0]);

/// Storage slot for `blobBaseFee` in the `L1GasPriceOracle` contract.
/// Added in the Curie fork.
pub const GPO_L1_BLOB_BASE_FEE_SLOT: U256 = U256::from_limbs([5, 0, 0, 0]);

/// Storage slot for `commitScalar` in the `L1GasPriceOracle` contract.
/// Added in the Curie fork.
pub const GPO_COMMIT_SCALAR_SLOT: U256 = U256::from_limbs([6, 0, 0, 0]);

/// Storage slot for `blobScalar` in the `L1GasPriceOracle` contract.
/// Added in the Curie fork.
pub const GPO_BLOB_SCALAR_SLOT: U256 = U256::from_limbs([7, 0, 0, 0]);

/// Storage slot for `isCurie` in the `L1GasPriceOracle` contract.
/// Added in the Curie fork.
pub const GPO_IS_CURIE_SLOT: U256 = U256::from_limbs([8, 0, 0, 0]);

/// Storage slot for `penaltyThreshold` in the `L1GasPriceOracle` contract.
/// Added in the Feynman fork.
pub const GPO_PENALTY_THRESHOLD_SLOT: U256 = U256::from_limbs([9, 0, 0, 0]);

/// Storage slot for `penaltyFactor` in the `L1GasPriceOracle` contract.
/// Added in the Feynman fork.
pub const GPO_PENALTY_FACTOR_SLOT: U256 = U256::from_limbs([10, 0, 0, 0]);

/// Storage slot for `isFeynman` in the `L1GasPriceOracle` contract.
/// Added in the Feynman fork.
pub const GPO_IS_FEYNMAN_SLOT: U256 = U256::from_limbs([11, 0, 0, 0]);

/// Storage slot for `isGalileo` in the `L1GasPriceOracle` contract.
/// Added in the Galileo fork.
pub const GPO_IS_GALILEO_SLOT: U256 = U256::from_limbs([12, 0, 0, 0]);
