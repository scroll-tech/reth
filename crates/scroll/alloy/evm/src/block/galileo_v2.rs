//! `GalileoV2` fork transition for Scroll.

use alloc::vec;
use revm::{
    bytecode::Bytecode,
    database::{states::StorageSlot, State},
    primitives::{bytes, Bytes, U256},
    state::AccountInfo,
    Database,
};

// Import L1GasPriceOracle address and slots.
use crate::gas_price_oracle::*;

/// Bytecode of `L1GasPriceOracle` at `GalileoV2` transition.
/// Run these commands in the scroll-contracts repo to verify this bytecode:
///
/// git checkout dfffa0f04bbd1de31ef342e1642a2f9ad9a620fe
/// yarn
/// forge build
/// cat artifacts/src/L1GasPriceOracle.sol/L1GasPriceOracle.json | jq -r .deployedBytecode.object
const GALILEO_V2_L1_GAS_PRICE_ORACLE_BYTECODE: Bytes = bytes!("608060405234801561000f575f80fd5b50600436106101c6575f3560e01c8063715018a6116100fe578063bede39b51161009e578063e88a60ad1161006e578063e88a60ad1461035d578063f2fde38b14610370578063f45e65d814610383578063fe5b04151461038c575f80fd5b8063bede39b51461031c578063c63b9e2d1461032f578063c91e514914610342578063de26c4a11461034a575f80fd5b80638da5cb5b116100d95780638da5cb5b146102c457806393e59dc1146102ee578063944b247f14610301578063a911d77f14610314575f80fd5b8063715018a6146102ab5780637f977cbf146102b357806384189161146102bb575f80fd5b80633d0f963e116101695780635471db39116101445780635471db391461027d5780636112d6db146102865780636a5e67e51461028f5780637046559714610298575f80fd5b80633d0f963e1461024e57806349948e0e14610261578063519b4bd314610274575f80fd5b806323e524ac116101a457806323e524ac146102105780633577afc51461021957806339455d3a1461022e5780633b7656bb14610241575f80fd5b80630c18c162146101ca5780630f337f6d146101e657806313dad5be14610203575b5f80fd5b6101d360025481565b6040519081526020015b60405180910390f35b600c546101f39060ff1681565b60405190151581526020016101dd565b6008546101f39060ff1681565b6101d360065481565b61022c610227366004610ccf565b610394565b005b61022c61023c366004610ce6565b610426565b600b546101f39060ff1681565b61022c61025c366004610d06565b610523565b6101d361026f366004610d47565b6105a6565b6101d360015481565b6101d360095481565b6101d3600a5481565b6101d360075481565b61022c6102a6366004610ccf565b6105f3565b61022c610681565b61022c6106b5565b6101d360055481565b5f546102d6906001600160a01b031681565b6040516001600160a01b0390911681526020016101dd565b6004546102d6906001600160a01b031681565b61022c61030f366004610ccf565b610711565b61022c61079d565b61022c61032a366004610ccf565b6107f9565b61022c61033d366004610ccf565b6108b6565b6009546101d3565b6101d3610358366004610d47565b610933565b61022c61036b366004610ccf565b61096a565b61022c61037e366004610d06565b6109f6565b6101d360035481565b61022c610a81565b5f546001600160a01b031633146103c65760405162461bcd60e51b81526004016103bd90610df2565b60405180910390fd5b621c9c388111156103ea57604051635742c80560e11b815260040160405180910390fd5b60028190556040518181527f32740b35c0ea213650f60d44366b4fb211c9033b50714e4a1d34e65d5beb9bb4906020015b60405180910390a150565b6004805460405163efc7840160e01b815233928101929092526001600160a01b03169063efc7840190602401602060405180830381865afa15801561046d573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906104919190610e29565b6104ae576040516326b3506d60e11b815260040160405180910390fd5b600182905560058190556040518281527f351fb23757bb5ea0546c85b7996ddd7155f96b939ebaa5ff7bc49c75f27f2c449060200160405180910390a16040518181527f9a14bfb5d18c4c3cf14cae19c23d7cf1bcede357ea40ca1f75cd49542c71c214906020015b60405180910390a15050565b5f546001600160a01b0316331461054c5760405162461bcd60e51b81526004016103bd90610df2565b600480546001600160a01b038381166001600160a01b031983168117909355604080519190921680825260208201939093527f22d1c35fe072d2e42c3c8f9bd4a0d34aa84a0101d020a62517b33fdb3174e5f79101610517565b600c545f9060ff16156105c2576105bc82610add565b92915050565b600b5460ff16156105d6576105bc82610b55565b60085460ff16156105ea576105bc82610bb3565b6105bc82610bef565b5f546001600160a01b0316331461061c5760405162461bcd60e51b81526004016103bd90610df2565b61062c633b9aca006103e8610e5c565b81111561064c57604051631e44fdeb60e11b815260040160405180910390fd5b60038190556040518181527f3336cd9708eaf2769a0f0dc0679f30e80f15dcd88d1921b5a16858e8b85c591a9060200161041b565b5f546001600160a01b031633146106aa5760405162461bcd60e51b81526004016103bd90610df2565b6106b35f610c20565b565b5f546001600160a01b031633146106de5760405162461bcd60e51b81526004016103bd90610df2565b600c5460ff16156107025760405163182389a760e01b815260040160405180910390fd5b600c805460ff19166001179055565b5f546001600160a01b0316331461073a5760405162461bcd60e51b81526004016103bd90610df2565b610748633b9aca0080610e5c565b8111156107685760405163874f603160e01b815260040160405180910390fd5b60068190556040518181527f2ab3f5a4ebbcbf3c24f62f5454f52f10e1a8c9dcc5acac8f19199ce881a6a1089060200161041b565b5f546001600160a01b031633146107c65760405162461bcd60e51b81526004016103bd90610df2565b60085460ff16156107ea576040516379f9c57560e01b815260040160405180910390fd5b6008805460ff19166001179055565b6004805460405163efc7840160e01b815233928101929092526001600160a01b03169063efc7840190602401602060405180830381865afa158015610840573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906108649190610e29565b610881576040516326b3506d60e11b815260040160405180910390fd5b60018190556040518181527f351fb23757bb5ea0546c85b7996ddd7155f96b939ebaa5ff7bc49c75f27f2c449060200161041b565b5f546001600160a01b031633146108df5760405162461bcd60e51b81526004016103bd90610df2565b805f036108fe5760405162ae184360e01b815260040160405180910390fd5b600a8190556040518181527f8647cebb7e57360673a28415c0bed2f68c42a86c5035f1c9b2eda2b09509288a9060200161041b565b600c545f9060ff16806109485750600b5460ff165b80610955575060085460ff165b1561096157505f919050565b6105bc82610c6f565b5f546001600160a01b031633146109935760405162461bcd60e51b81526004016103bd90610df2565b6109a1633b9aca0080610e5c565b8111156109c15760405163f37ec21560e01b815260040160405180910390fd5b60078190556040518181527f6b332a036d8c3ead57dcb06c87243bd7a2aed015ddf2d0528c2501dae56331aa9060200161041b565b5f546001600160a01b03163314610a1f5760405162461bcd60e51b81526004016103bd90610df2565b6001600160a01b038116610a755760405162461bcd60e51b815260206004820152601d60248201527f6e6577206f776e657220697320746865207a65726f206164647265737300000060448201526064016103bd565b610a7e81610c20565b50565b5f546001600160a01b03163314610aaa5760405162461bcd60e51b81526004016103bd90610df2565b600b5460ff1615610ace57604051631a7c228b60e21b815260040160405180910390fd5b600b805460ff19166001179055565b5f808251600554600754610af19190610e5c565b600154600654610b019190610e5c565b610b0b9190610e73565b610b159190610e5c565b90505f600a54845183610b289190610e5c565b610b329190610e86565b9050633b9aca00610b438284610e73565b610b4d9190610e86565b949350505050565b5f633b9aca0080600a548451600554600754610b719190610e5c565b600154600654610b819190610e5c565b610b8b9190610e73565b610b959190610e5c565b610b9f9190610e5c565b610ba99190610e86565b6105bc9190610e86565b5f633b9aca006005548351600754610bcb9190610e5c565b610bd59190610e5c565b600154600654610be59190610e5c565b610ba99190610e73565b5f80610bfa83610c6f565b90505f60015482610c0b9190610e5c565b9050633b9aca0060035482610b439190610e5c565b5f80546001600160a01b038381166001600160a01b0319831681178455604051919092169283917f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e09190a35050565b80515f908190815b81811015610cc057848181518110610c9157610c91610ea5565b01602001516001600160f81b0319165f03610cb157600483019250610cb8565b6010830192505b600101610c77565b50506002540160400192915050565b5f60208284031215610cdf575f80fd5b5035919050565b5f8060408385031215610cf7575f80fd5b50508035926020909101359150565b5f60208284031215610d16575f80fd5b81356001600160a01b0381168114610d2c575f80fd5b9392505050565b634e487b7160e01b5f52604160045260245ffd5b5f60208284031215610d57575f80fd5b813567ffffffffffffffff80821115610d6e575f80fd5b818401915084601f830112610d81575f80fd5b813581811115610d9357610d93610d33565b604051601f8201601f19908116603f01168101908382118183101715610dbb57610dbb610d33565b81604052828152876020848701011115610dd3575f80fd5b826020860160208301375f928101602001929092525095945050505050565b60208082526017908201527f63616c6c6572206973206e6f7420746865206f776e6572000000000000000000604082015260600190565b5f60208284031215610e39575f80fd5b81518015158114610d2c575f80fd5b634e487b7160e01b5f52601160045260245ffd5b80820281158282048414176105bc576105bc610e48565b808201808211156105bc576105bc610e48565b5f82610ea057634e487b7160e01b5f52601260045260245ffd5b500490565b634e487b7160e01b5f52603260045260245ffdfea164736f6c6343000818000a");

/// Galileo slot is set to 1 (true) after the `GalileoV2` block fork.
const IS_GALILEO: U256 = U256::from_limbs([1, 0, 0, 0]);

/// Storage update of L1 gas price oracle at `GalileoV2` transition.
const GALILEO_V2_L1_GAS_PRICE_ORACLE_STORAGE: [(U256, U256); 1] =
    [(GPO_IS_GALILEO_SLOT, IS_GALILEO)];

/// Applies the Scroll `GalileoV2` hard fork to the state:
///    - Updates the L1 oracle contract bytecode.
///    - Sets the `isGalileo` slot to 1 (true).
pub(super) fn apply_galileo_v2_hard_fork<DB: Database>(
    state: &mut State<DB>,
) -> Result<(), DB::Error> {
    // No-op if already applied.
    // Note: This requires a storage read for every block after `GalileoV2`, and it means this
    // read needs to be included in the execution witness. Unfortunately, there is no
    // other reliable way to apply the change only at the transition block, since
    // `ScrollBlockExecutor` does not have access to the parent timestamp.
    if state.storage(L1_GAS_PRICE_ORACLE_ADDRESS, GPO_IS_GALILEO_SLOT)? == IS_GALILEO {
        return Ok(())
    }

    let oracle = state.load_cache_account(L1_GAS_PRICE_ORACLE_ADDRESS)?;

    // compute the code hash
    let bytecode = Bytecode::new_raw(GALILEO_V2_L1_GAS_PRICE_ORACLE_BYTECODE);
    let code_hash = bytecode.hash_slow();

    // get the old oracle account info
    let old_oracle_info = oracle.account_info().unwrap_or_default();

    // init new oracle account information
    let new_oracle_info = AccountInfo { code_hash, code: Some(bytecode), ..old_oracle_info };

    // init new storage
    let new_storage = GALILEO_V2_L1_GAS_PRICE_ORACLE_STORAGE
        .into_iter()
        .map(|(slot, present_value)| {
            (
                slot,
                StorageSlot {
                    present_value,
                    previous_or_original_value: oracle.storage_slot(slot).unwrap_or_default(),
                },
            )
        })
        .collect();

    // create transition for oracle new account info and storage
    let transition = oracle.change(new_oracle_info, new_storage);

    // add transition
    if let Some(s) = state.transition_state.as_mut() {
        s.add_transitions(vec![(L1_GAS_PRICE_ORACLE_ADDRESS, transition)])
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feynman::FEYNMAN_L1_GAS_PRICE_ORACLE_BYTECODE;
    use revm::{
        database::{
            states::{bundle_state::BundleRetention, plain_account::PlainStorage, StorageSlot},
            CacheDB, EmptyDB, State,
        },
        primitives::{keccak256, U256},
        state::{AccountInfo, Bytecode},
        Database,
    };
    use std::str::FromStr;

    #[test]
    fn test_apply_galileo_v2_fork() -> eyre::Result<()> {
        // init state
        let db = EmptyDB::new();
        let mut state =
            State::builder().with_database(db).with_bundle_update().without_state_clear().build();

        // oracle pre fork state
        let bytecode_pre_fork = Bytecode::new_raw(FEYNMAN_L1_GAS_PRICE_ORACLE_BYTECODE);
        let oracle_pre_fork = AccountInfo {
            code_hash: bytecode_pre_fork.hash_slow(),
            code: Some(bytecode_pre_fork),
            ..Default::default()
        };
        let oracle_storage_pre_fork = PlainStorage::from_iter([
            (GPO_OWNER_SLOT, U256::from_str("0x13d24a7ff6f5ec5ff0e9c40fc3b8c9c01c65437b")?),
            (GPO_L1_BASE_FEE_SLOT, U256::from(0x15f50e5e)),
            (GPO_OVERHEAD_SLOT, U256::from(0x38)),
            (GPO_SCALAR_SLOT, U256::from(0x3e95ba80)),
            (GPO_WHITELIST_SLOT, U256::from_str("0x5300000000000000000000000000000000000003")?),
            (GPO_L1_BLOB_BASE_FEE_SLOT, U256::from(0x15f50e5e)),
            (GPO_COMMIT_SCALAR_SLOT, U256::from(0x3e95ba80)),
            (GPO_BLOB_SCALAR_SLOT, U256::from(0x3e95ba80)),
            (GPO_IS_CURIE_SLOT, U256::from(1)),
            (GPO_PENALTY_THRESHOLD_SLOT, U256::from(1_000_000_000)),
            (GPO_PENALTY_FACTOR_SLOT, U256::from(1_000_000_000)),
            (GPO_IS_FEYNMAN_SLOT, U256::from(1)),
        ]);
        state.insert_account_with_storage(
            L1_GAS_PRICE_ORACLE_ADDRESS,
            oracle_pre_fork.clone(),
            oracle_storage_pre_fork.clone(),
        );

        // apply GalileoV2 fork
        apply_galileo_v2_hard_fork(&mut state)?;

        // merge transitions
        state.merge_transitions(BundleRetention::Reverts);
        let bundle = state.take_bundle();

        // check oracle account info
        let oracle = bundle.state.get(&L1_GAS_PRICE_ORACLE_ADDRESS).unwrap().clone();
        let code_hash = keccak256(&GALILEO_V2_L1_GAS_PRICE_ORACLE_BYTECODE);
        let bytecode = Bytecode::new_raw(GALILEO_V2_L1_GAS_PRICE_ORACLE_BYTECODE);
        let expected_oracle_info =
            AccountInfo { code_hash, code: Some(bytecode.clone()), ..Default::default() };

        assert_eq!(oracle.original_info.unwrap(), oracle_pre_fork);
        assert_eq!(oracle.info.unwrap(), expected_oracle_info);

        // check oracle storage changeset
        let mut storage = oracle.storage.into_iter().collect::<Vec<(U256, StorageSlot)>>();
        storage.sort_by_key(|(a, _)| *a);
        for (got, expected) in storage.into_iter().zip(GALILEO_V2_L1_GAS_PRICE_ORACLE_STORAGE) {
            assert_eq!(got.0, expected.0);
            assert_eq!(got.1, StorageSlot { present_value: expected.1, ..Default::default() });
        }

        // check oracle original storage
        for (slot, value) in oracle_storage_pre_fork {
            assert_eq!(state.storage(L1_GAS_PRICE_ORACLE_ADDRESS, slot)?, value)
        }

        // check deployed contract
        assert_eq!(bundle.contracts.get(&code_hash).unwrap(), &bytecode);

        Ok(())
    }

    #[test]
    fn test_apply_galileo_v2_fork_only_once() -> eyre::Result<()> {
        let bytecode = Bytecode::new_raw(GALILEO_V2_L1_GAS_PRICE_ORACLE_BYTECODE);

        let oracle_account = AccountInfo {
            code_hash: bytecode.hash_slow(),
            code: Some(bytecode),
            ..Default::default()
        };

        let oracle_storage = PlainStorage::from_iter([
            (GPO_OWNER_SLOT, U256::from_str("0x13d24a7ff6f5ec5ff0e9c40fc3b8c9c01c65437b")?),
            (GPO_L1_BASE_FEE_SLOT, U256::from(0x15f50e5e)),
            (GPO_OVERHEAD_SLOT, U256::from(0x38)),
            (GPO_SCALAR_SLOT, U256::from(0x3e95ba80)),
            (GPO_WHITELIST_SLOT, U256::from_str("0x5300000000000000000000000000000000000003")?),
            (GPO_L1_BLOB_BASE_FEE_SLOT, U256::from(0x15f50e5e)),
            (GPO_COMMIT_SCALAR_SLOT, U256::from(0x3e95ba80)),
            (GPO_BLOB_SCALAR_SLOT, U256::from(0x3e95ba80)),
            (GPO_IS_CURIE_SLOT, U256::from(1)),
            (GPO_PENALTY_THRESHOLD_SLOT, U256::from(1_100_000_000u64)),
            (GPO_PENALTY_FACTOR_SLOT, U256::from(3_000_000_000u64)),
            (GPO_IS_FEYNMAN_SLOT, U256::from(1)),
            (GPO_IS_GALILEO_SLOT, U256::from(1)),
        ]);

        // init state,
        // we write to db directly to make sure we do not have account storage in cache
        let mut db = CacheDB::new(EmptyDB::default());

        db.insert_account_info(L1_GAS_PRICE_ORACLE_ADDRESS, oracle_account);

        for (slot, value) in oracle_storage {
            db.insert_account_storage(L1_GAS_PRICE_ORACLE_ADDRESS, slot, value).unwrap();
        }

        let mut state =
            State::builder().with_database(db).with_bundle_update().without_state_clear().build();

        // make sure account is in cache
        state.load_cache_account(L1_GAS_PRICE_ORACLE_ADDRESS)?;

        // apply GalileoV2 fork
        apply_galileo_v2_hard_fork(&mut state)?;

        // merge transitions
        state.merge_transitions(BundleRetention::Reverts);
        let bundle = state.take_bundle();

        // isGalileo is already set, apply_galileo_v2_hard_fork should be a no-op
        assert_eq!(bundle.state.get(&L1_GAS_PRICE_ORACLE_ADDRESS), None);

        Ok(())
    }
}
