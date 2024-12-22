# scroll-alloy-consensus

Scroll consensus interface.

This crate contains constants, types, and functions for implementing Scroll EL consensus and communication. This
includes an extended `ScrollTxEnvelope` type with deposit transactions

In general a type belongs in this crate if it exists in the `alloy-consensus` crate, but was modified from the base Ethereum protocol in the OP Stack.
For consensus types that are not modified by the Scroll, the `alloy-consensus` types should be used instead.

## Provenance

Much of this code was ported from [reth-primitives] as part of ongoing alloy migrations.

[reth-primitives]: https://github.com/paradigmxyz/reth/tree/main/crates/primitives
