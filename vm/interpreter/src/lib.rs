// Copyright 2019-2023 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

mod fvm;
pub mod fvm3;
#[cfg(feature = "instrumented_kernel")]
mod instrumented_kernel;
#[cfg(feature = "instrumented_kernel")]
mod metrics;
mod vm;

use forest_actor_interface::account;
use forest_shim::{
    address::{Address, Protocol},
    state_tree::StateTree,
};
use fvm_ipld_blockstore::Blockstore;

pub use self::vm::*;

/// returns the public key type of address (`BLS`/`SECP256K1`) of an account
/// actor identified by `addr`.
pub fn resolve_to_key_addr<BS, S>(
    st: &StateTree<S>,
    store: &BS,
    addr: &Address,
) -> Result<Address, anyhow::Error>
where
    BS: Blockstore,
    S: Blockstore + Clone,
{
    if addr.protocol() == Protocol::BLS || addr.protocol() == Protocol::Secp256k1 {
        return Ok(*addr);
    }

    let act = st
        .get_actor(addr)?
        .ok_or_else(|| anyhow::anyhow!("Failed to retrieve actor: {}", addr))?;

    let acc_st = account::State::load(store, &act)?;

    Ok(acc_st.pubkey_address().into())
}
