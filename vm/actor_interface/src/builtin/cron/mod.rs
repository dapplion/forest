// Copyright 2019-2023 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

use forest_shim::address::Address;

/// Cron actor address.
pub const ADDRESS: Address = Address::new_id(3);

/// Cron actor method.
pub type Method = fil_actor_cron_v8::Method;
