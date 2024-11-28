use revm::{
    db::{states::StorageSlot, StorageWithOriginalValues},
    primitives::{address, bytes, Address, Bytecode, Bytes, U256},
    shared::AccountInfo,
    Database, State,
};

const L1_GAS_PRICE_ORACLE_ADDRESS: Address = address!("5300000000000000000000000000000000000002");
const CURIE_L1_GAS_PRICE_ORACLE_BYTECODE: Bytes = bytes!("608060405234801561000f575f80fd5b5060043610610132575f3560e01c8063715018a6116100b4578063a911d77f11610079578063a911d77f1461024c578063bede39b514610254578063de26c4a114610267578063e88a60ad1461027a578063f2fde38b1461028d578063f45e65d8146102a0575f80fd5b8063715018a6146101eb57806384189161146101f35780638da5cb5b146101fc57806393e59dc114610226578063944b247f14610239575f80fd5b80633d0f963e116100fa5780633d0f963e146101a057806349948e0e146101b3578063519b4bd3146101c65780636a5e67e5146101cf57806370465597146101d8575f80fd5b80630c18c1621461013657806313dad5be1461015257806323e524ac1461016f5780633577afc51461017857806339455d3a1461018d575b5f80fd5b61013f60025481565b6040519081526020015b60405180910390f35b60085461015f9060ff1681565b6040519015158152602001610149565b61013f60065481565b61018b6101863660046109b3565b6102a9565b005b61018b61019b3660046109ca565b61033b565b61018b6101ae3660046109ea565b610438565b61013f6101c1366004610a2b565b6104bb565b61013f60015481565b61013f60075481565b61018b6101e63660046109b3565b6104e0565b61018b61056e565b61013f60055481565b5f5461020e906001600160a01b031681565b6040516001600160a01b039091168152602001610149565b60045461020e906001600160a01b031681565b61018b6102473660046109b3565b6105a2565b61018b61062e565b61018b6102623660046109b3565b61068a565b61013f610275366004610a2b565b610747565b61018b6102883660046109b3565b610764565b61018b61029b3660046109ea565b6107f0565b61013f60035481565b5f546001600160a01b031633146102db5760405162461bcd60e51b81526004016102d290610ad6565b60405180910390fd5b621c9c388111156102ff57604051635742c80560e11b815260040160405180910390fd5b60028190556040518181527f32740b35c0ea213650f60d44366b4fb211c9033b50714e4a1d34e65d5beb9bb4906020015b60405180910390a150565b6004805460405163efc7840160e01b815233928101929092526001600160a01b03169063efc7840190602401602060405180830381865afa158015610382573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906103a69190610b0d565b6103c3576040516326b3506d60e11b815260040160405180910390fd5b600182905560058190556040518281527f351fb23757bb5ea0546c85b7996ddd7155f96b939ebaa5ff7bc49c75f27f2c449060200160405180910390a16040518181527f9a14bfb5d18c4c3cf14cae19c23d7cf1bcede357ea40ca1f75cd49542c71c214906020015b60405180910390a15050565b5f546001600160a01b031633146104615760405162461bcd60e51b81526004016102d290610ad6565b600480546001600160a01b038381166001600160a01b031983168117909355604080519190921680825260208201939093527f22d1c35fe072d2e42c3c8f9bd4a0d34aa84a0101d020a62517b33fdb3174e5f7910161042c565b6008545f9060ff16156104d7576104d18261087b565b92915050565b6104d1826108c1565b5f546001600160a01b031633146105095760405162461bcd60e51b81526004016102d290610ad6565b610519633b9aca006103e8610b40565b81111561053957604051631e44fdeb60e11b815260040160405180910390fd5b60038190556040518181527f3336cd9708eaf2769a0f0dc0679f30e80f15dcd88d1921b5a16858e8b85c591a90602001610330565b5f546001600160a01b031633146105975760405162461bcd60e51b81526004016102d290610ad6565b6105a05f610904565b565b5f546001600160a01b031633146105cb5760405162461bcd60e51b81526004016102d290610ad6565b6105d9633b9aca0080610b40565b8111156105f95760405163874f603160e01b815260040160405180910390fd5b60068190556040518181527f2ab3f5a4ebbcbf3c24f62f5454f52f10e1a8c9dcc5acac8f19199ce881a6a10890602001610330565b5f546001600160a01b031633146106575760405162461bcd60e51b81526004016102d290610ad6565b60085460ff161561067b576040516379f9c57560e01b815260040160405180910390fd5b6008805460ff19166001179055565b6004805460405163efc7840160e01b815233928101929092526001600160a01b03169063efc7840190602401602060405180830381865afa1580156106d1573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906106f59190610b0d565b610712576040516326b3506d60e11b815260040160405180910390fd5b60018190556040518181527f351fb23757bb5ea0546c85b7996ddd7155f96b939ebaa5ff7bc49c75f27f2c4490602001610330565b6008545f9060ff161561075b57505f919050565b6104d182610953565b5f546001600160a01b0316331461078d5760405162461bcd60e51b81526004016102d290610ad6565b61079b633b9aca0080610b40565b8111156107bb5760405163f37ec21560e01b815260040160405180910390fd5b60078190556040518181527f6b332a036d8c3ead57dcb06c87243bd7a2aed015ddf2d0528c2501dae56331aa90602001610330565b5f546001600160a01b031633146108195760405162461bcd60e51b81526004016102d290610ad6565b6001600160a01b03811661086f5760405162461bcd60e51b815260206004820152601d60248201527f6e6577206f776e657220697320746865207a65726f206164647265737300000060448201526064016102d2565b61087881610904565b50565b5f633b9aca0060055483516007546108939190610b40565b61089d9190610b40565b6001546006546108ad9190610b40565b6108b79190610b57565b6104d19190610b6a565b5f806108cc83610953565b90505f600154826108dd9190610b40565b9050633b9aca00600354826108f29190610b40565b6108fc9190610b6a565b949350505050565b5f80546001600160a01b038381166001600160a01b0319831681178455604051919092169283917f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e09190a35050565b80515f908190815b818110156109a45784818151811061097557610975610b89565b01602001516001600160f81b0319165f036109955760048301925061099c565b6010830192505b60010161095b565b50506002540160400192915050565b5f602082840312156109c3575f80fd5b5035919050565b5f80604083850312156109db575f80fd5b50508035926020909101359150565b5f602082840312156109fa575f80fd5b81356001600160a01b0381168114610a10575f80fd5b9392505050565b634e487b7160e01b5f52604160045260245ffd5b5f60208284031215610a3b575f80fd5b813567ffffffffffffffff80821115610a52575f80fd5b818401915084601f830112610a65575f80fd5b813581811115610a7757610a77610a17565b604051601f8201601f19908116603f01168101908382118183101715610a9f57610a9f610a17565b81604052828152876020848701011115610ab7575f80fd5b826020860160208301375f928101602001929092525095945050505050565b60208082526017908201527f63616c6c6572206973206e6f7420746865206f776e6572000000000000000000604082015260600190565b5f60208284031215610b1d575f80fd5b81518015158114610a10575f80fd5b634e487b7160e01b5f52601160045260245ffd5b80820281158282048414176104d1576104d1610b2c565b808201808211156104d1576104d1610b2c565b5f82610b8457634e487b7160e01b5f52601260045260245ffd5b500490565b634e487b7160e01b5f52603260045260245ffdfea26469706673582212200c2ac583f18be4f94ab169ae6f2ea3a708a7c0d4424746b120b177adb39e626064736f6c63430008180033");
const CURIE_L1_GAS_PRICE_ORACLE_STORAGE: [(U256, U256); 4] = [
    (L1_BLOB_BASE_FEE_SLOT, INITIAL_L1_BLOB_BASE_FEE),
    (COMMIT_SCALAR_SLOT, INITIAL_COMMIT_SCALAR),
    (BLOB_SCALAR_SLOT, INITIAL_BLOB_SCALAR),
    (IS_CURIE_SLOT, IS_CURIE),
];

const L1_BLOB_BASE_FEE_SLOT: U256 = U256::from_limbs([5, 0, 0, 0]);
const INITIAL_L1_BLOB_BASE_FEE: U256 = U256::from_limbs([1, 0, 0, 0]);
const COMMIT_SCALAR_SLOT: U256 = U256::from_limbs([6, 0, 0, 0]);
const INITIAL_COMMIT_SCALAR: U256 = U256::from_limbs([230759955285, 0, 0, 0]);
const BLOB_SCALAR_SLOT: U256 = U256::from_limbs([7, 0, 0, 0]);
const INITIAL_BLOB_SCALAR: U256 = U256::from_limbs([417565260, 0, 0, 0]);
const IS_CURIE_SLOT: U256 = U256::from_limbs([8, 0, 0, 0]);
const IS_CURIE: U256 = U256::from_limbs([1, 0, 0, 0]);

/// Applies the Scroll curie hard fork to the state.
pub fn apply_curie_hard_fork<DB: Database>(state: &mut State<DB>) -> Result<(), DB::Error> {
    let oracle = state.load_cache_account(L1_GAS_PRICE_ORACLE_ADDRESS)?;

    // get old oracle account balance and nonce
    let (old_balance, old_nonce) =
        oracle.account.as_ref().map(|acc| (acc.info.balance, acc.info.nonce)).unwrap_or_default();

    // compute the code hash
    let bytecode = Bytecode::new_raw(CURIE_L1_GAS_PRICE_ORACLE_BYTECODE);
    let code_hash = bytecode.hash_slow();

    // init new oracle account information
    let new_oracle_info =
        AccountInfo { balance: old_balance, nonce: old_nonce, code_hash, code: Some(bytecode) };

    // create `StorageWithOriginalValues` mapping for oracle
    let storage = oracle
        .account
        .as_ref()
        .map(|acc| {
            let mut storage_with_original_values = StorageWithOriginalValues::default();
            for (slot, new_storage_value) in CURIE_L1_GAS_PRICE_ORACLE_STORAGE {
                storage_with_original_values.insert(
                    slot,
                    StorageSlot {
                        previous_or_original_value: acc
                            .storage
                            .get(&slot)
                            .copied()
                            .unwrap_or_default(),
                        present_value: new_storage_value,
                    },
                );
            }
            storage_with_original_values
        })
        .unwrap_or_default();

    // create transition for oracle new account info and storage
    let transition = oracle.change(new_oracle_info, storage);

    // append transition
    if let Some(s) = state.transition_state.as_mut() {
        s.add_transitions(vec![(L1_GAS_PRICE_ORACLE_ADDRESS, transition)])
    }

    Ok(())
}
