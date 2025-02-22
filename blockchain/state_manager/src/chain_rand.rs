// Copyright 2019-2023 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

use std::{io::Write, sync::Arc};

use anyhow::{bail, Context};
use blake2b_simd::Params;
use byteorder::{BigEndian, WriteBytesExt};
use forest_beacon::{Beacon, BeaconEntry, BeaconSchedule, DrandBeacon};
use forest_blocks::{Tipset, TipsetKeys};
use forest_chain::ChainStore;
use forest_db::Store;
use forest_encoding::blake2b_256;
use forest_networks::ChainConfig;
use fvm::externs::Rand as Rand_v2;
use fvm3::externs::Rand as Rand_v3;
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::clock::ChainEpoch;

/// Allows for deriving the randomness from a particular tipset.
pub struct ChainRand<DB> {
    chain_config: Arc<ChainConfig>,
    blks: TipsetKeys,
    cs: Arc<ChainStore<DB>>,
    beacon: Arc<BeaconSchedule<DrandBeacon>>,
}

impl<DB> Clone for ChainRand<DB> {
    fn clone(&self) -> Self {
        ChainRand {
            chain_config: self.chain_config.clone(),
            blks: self.blks.clone(),
            cs: self.cs.clone(),
            beacon: self.beacon.clone(),
        }
    }
}

impl<DB> ChainRand<DB>
where
    DB: Blockstore + Store + Send + Sync,
{
    pub fn new(
        chain_config: Arc<ChainConfig>,
        blks: TipsetKeys,
        cs: Arc<ChainStore<DB>>,
        beacon: Arc<BeaconSchedule<DrandBeacon>>,
    ) -> Self {
        Self {
            chain_config,
            blks,
            cs,
            beacon,
        }
    }

    /// Gets 32 bytes of randomness for `ChainRand` parameterized by the
    /// `DomainSeparationTag`, `ChainEpoch`, Entropy from the ticket chain.
    pub fn get_chain_randomness(
        &self,
        blocks: &TipsetKeys,
        pers: i64,
        round: ChainEpoch,
        entropy: &[u8],
        lookback: bool,
    ) -> anyhow::Result<[u8; 32]> {
        let ts = self.cs.tipset_from_keys(blocks)?;

        if round > ts.epoch() {
            bail!("cannot draw randomness from the future");
        }

        let search_height = if round < 0 { 0 } else { round };

        let rand_ts = self.cs.tipset_by_height(search_height, ts, lookback)?;

        draw_randomness(
            rand_ts
                .min_ticket()
                .context("No ticket exists for block")?
                .vrfproof
                .as_bytes(),
            pers,
            round,
            entropy,
        )
    }

    /// network version 13 onward
    pub fn get_chain_randomness_v2(
        &self,
        blocks: &TipsetKeys,
        pers: i64,
        round: ChainEpoch,
        entropy: &[u8],
    ) -> anyhow::Result<[u8; 32]> {
        self.get_chain_randomness(blocks, pers, round, entropy, false)
    }

    /// network version 13; without look-back
    pub fn get_beacon_randomness_v2(
        &self,
        blocks: &TipsetKeys,
        pers: i64,
        round: ChainEpoch,
        entropy: &[u8],
    ) -> anyhow::Result<[u8; 32]> {
        self.get_beacon_randomness(blocks, pers, round, entropy, false)
    }

    /// network version 14 onward
    pub fn get_beacon_randomness_v3(
        &self,
        blocks: &TipsetKeys,
        pers: i64,
        round: ChainEpoch,
        entropy: &[u8],
    ) -> anyhow::Result<[u8; 32]> {
        if round < 0 {
            return self.get_beacon_randomness_v2(blocks, pers, round, entropy);
        }

        let beacon_entry = self.extract_beacon_entry_for_epoch(blocks, round)?;
        draw_randomness(beacon_entry.data(), pers, round, entropy)
    }

    /// Gets 32 bytes of randomness for `ChainRand` parameterized by the
    /// `DomainSeparationTag`, `ChainEpoch`, Entropy from the latest beacon
    /// entry.
    pub fn get_beacon_randomness(
        &self,
        blocks: &TipsetKeys,
        pers: i64,
        round: ChainEpoch,
        entropy: &[u8],
        lookback: bool,
    ) -> anyhow::Result<[u8; 32]> {
        let rand_ts: Arc<Tipset> = self.get_beacon_randomness_tipset(blocks, round, lookback)?;
        let be = self.cs.latest_beacon_entry(&rand_ts)?;
        draw_randomness(be.data(), pers, round, entropy)
    }

    pub fn extract_beacon_entry_for_epoch(
        &self,
        blocks: &TipsetKeys,
        epoch: ChainEpoch,
    ) -> anyhow::Result<BeaconEntry> {
        let mut rand_ts: Arc<Tipset> = self.get_beacon_randomness_tipset(blocks, epoch, false)?;
        let (_, beacon) = self.beacon.beacon_for_epoch(epoch)?;
        let round =
            beacon.max_beacon_round_for_epoch(self.chain_config.network_version(epoch), epoch);

        for _ in 0..20 {
            let cbe = rand_ts.blocks()[0].beacon_entries();
            for v in cbe {
                if v.round() == round {
                    return Ok(v.clone());
                }
            }

            rand_ts = self.cs.tipset_from_keys(rand_ts.parents())?;
        }

        bail!(
            "didn't find beacon for round {:?} (epoch {:?})",
            round,
            epoch
        )
    }

    pub fn get_beacon_randomness_tipset(
        &self,
        blocks: &TipsetKeys,
        round: ChainEpoch,
        lookback: bool,
    ) -> anyhow::Result<Arc<Tipset>> {
        let ts = self.cs.tipset_from_keys(blocks)?;

        if round > ts.epoch() {
            bail!("cannot draw randomness from the future");
        }

        let search_height = if round < 0 { 0 } else { round };

        self.cs
            .tipset_by_height(search_height, ts, lookback)
            .map_err(|e| e.into())
    }
}

impl<DB> Rand_v2 for ChainRand<DB>
where
    DB: Blockstore + Store + Send + Sync,
{
    fn get_chain_randomness(
        &self,
        pers: i64,
        round: ChainEpoch,
        entropy: &[u8],
    ) -> anyhow::Result<[u8; 32]> {
        self.get_chain_randomness_v2(&self.blks, pers, round, entropy)
    }

    fn get_beacon_randomness(
        &self,
        pers: i64,
        round: ChainEpoch,
        entropy: &[u8],
    ) -> anyhow::Result<[u8; 32]> {
        self.get_beacon_randomness_v3(&self.blks, pers, round, entropy)
    }
}

impl<DB> Rand_v3 for ChainRand<DB>
where
    DB: Blockstore + Store + Send + Sync,
{
    fn get_chain_randomness(
        &self,
        pers: i64,
        round: ChainEpoch,
        entropy: &[u8],
    ) -> anyhow::Result<[u8; 32]> {
        self.get_chain_randomness_v2(&self.blks, pers, round, entropy)
    }

    fn get_beacon_randomness(
        &self,
        pers: i64,
        round: ChainEpoch,
        entropy: &[u8],
    ) -> anyhow::Result<[u8; 32]> {
        self.get_beacon_randomness_v3(&self.blks, pers, round, entropy)
    }
}

/// Computes a pseudo random 32 byte `Vec`.
pub fn draw_randomness(
    rbase: &[u8],
    pers: i64,
    round: ChainEpoch,
    entropy: &[u8],
) -> anyhow::Result<[u8; 32]> {
    let mut state = Params::new().hash_length(32).to_state();
    state.write_i64::<BigEndian>(pers)?;
    let vrf_digest = blake2b_256(rbase);
    state.write_all(&vrf_digest)?;
    state.write_i64::<BigEndian>(round)?;
    state.write_all(entropy)?;
    let mut ret = [0u8; 32];
    ret.clone_from_slice(state.finalize().as_bytes());
    Ok(ret)
}
