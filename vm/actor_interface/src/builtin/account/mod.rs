// Copyright 2019-2023 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::Context;
use cid::Cid;
use forest_shim::state_tree::ActorState;
use forest_utils::db::BlockstoreExt;
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::address::Address;
use serde::Serialize;

/// Account actor method.
pub type Method = fil_actor_account_v8::Method;

/// Account actor state.
#[derive(Serialize)]
#[serde(untagged)]
pub enum State {
    V8(fil_actor_account_v8::State),
    V9(fil_actor_account_v9::State),
    V10(fil_actor_account_v10::State),
}

pub fn is_v8_account_cid(cid: &Cid) -> bool {
    let known_cids = vec![
        // calibnet v8
        Cid::try_from("bafk2bzacecruossn66xqbeutqx5r4k2kjzgd43frmwd4qkw6haez44ubvvpxo").unwrap(),
        // mainnet
        Cid::try_from("bafk2bzacedudbf7fc5va57t3tmo63snmt3en4iaidv4vo3qlyacbxaa6hlx6y").unwrap(),
        // devnet
        Cid::try_from("bafk2bzacea4tlgnp7m6tlldpz3termlwxlnyq24nwd4zdzv4r6nsjuaktuuzc").unwrap(),
    ];
    known_cids.contains(cid)
}

pub fn is_v9_account_cid(cid: &Cid) -> bool {
    let known_cids = vec![
        // calibnet v9
        Cid::try_from("bafk2bzaceavfgpiw6whqigmskk74z4blm22nwjfnzxb4unlqz2e4wg3c5ujpw").unwrap(),
        // mainnet v9
        Cid::try_from("bafk2bzacect2p7urje3pylrrrjy3tngn6yaih4gtzauuatf2jllk3ksgfiw2y").unwrap(),
    ];
    known_cids.contains(cid)
}

pub fn is_v10_account_cid(cid: &Cid) -> bool {
    let known_cids = vec![
        // calibnet v10
        Cid::try_from("bafk2bzacebhfuz3sv7duvk653544xsxhdn4lsmy7ol7k6gdgancyctvmd7lnq").unwrap(),
        // mainnet v10
        Cid::try_from("bafk2bzacect72amqxedrtjymuq5lfrskk2itnniyfa5gdvqp5sjoeeb33oi2e").unwrap(),
    ];
    known_cids.contains(cid)
}

impl State {
    pub fn load<BS>(store: &BS, actor: &ActorState) -> anyhow::Result<State>
    where
        BS: Blockstore,
    {
        if is_v8_account_cid(&actor.code) {
            return store
                .get_obj(&actor.state)?
                .map(State::V8)
                .context("Actor state doesn't exist in store");
        }
        if is_v9_account_cid(&actor.code) {
            return store
                .get_obj(&actor.state)?
                .map(State::V9)
                .context("Actor state doesn't exist in store");
        }
        if is_v10_account_cid(&actor.code) {
            return store
                .get_obj(&actor.state)?
                .map(State::V10)
                .context("Actor state doesn't exist in store");
        }
        Err(anyhow::anyhow!("Unknown account actor code {}", actor.code))
    }

    pub fn pubkey_address(&self) -> Address {
        match self {
            State::V8(st) => st.address,
            State::V9(st) => st.address,
            State::V10(st) => st.address,
        }
    }
}
