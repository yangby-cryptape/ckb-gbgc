// Copyright (C) 2019 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub const SPECS_TEMPLATE: &str = include_str!("specs.toml");

pub const ROUND_1_AWARDS: &str = include_str!("competitions/round-1/awards.csv");
pub const ROUND_1_LOTTERY: &str = include_str!("competitions/round-1/lottery.csv");
pub const ROUND_2_MINED: &str = include_str!("competitions/round-2/miner_reward_finally.csv");
pub const ROUND_2_LUCKY: &str = include_str!("competitions/round-2/epoch_reward_finally.csv");
pub const ROUND_3_MINED: &str = include_str!("competitions/round-3/miner_reward.csv");
pub const ROUND_3_LUCKY: &str = include_str!("competitions/round-3/epoch_reward.csv");
pub const ROUND_4_MINED: &str = include_str!("competitions/round-4/miner_reward.csv");
pub const ROUND_5_S1_MINED: &str = include_str!("competitions/round-5/stage-1/miner_reward.csv");
pub const ROUND_5_S2_MINED: &str = include_str!("competitions/round-5/stage-2/miner_reward.csv");

pub const GENESIS_ALLOCATE: &str = include_str!("allocate/genesis_final.csv");
