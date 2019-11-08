// Copyright (C) 2019 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{collections::HashMap, sync::Arc, thread, time};

use futures::future::Future;
use parking_lot::RwLock;
use property::Property;
use tokio::runtime;

use uckb_jsonrpc_client::{
    client::{CkbAsyncClient, CkbSyncClient},
    interfaces::types::{core, packed, prelude::*, utilities, U256},
};

use crate::{
    arguments, constants,
    error::{Error, Result},
};

#[derive(Property)]
pub struct ChainData {
    rewards: HashMap<Vec<u8>, u64>,
    header: core::HeaderView,
    diff_avg: U256,
}

type BlockData = HashMap<u64, (Vec<u8>, u64)>;

fn sleep_millis(millis: u64) {
    let millis = time::Duration::from_millis(millis);
    thread::sleep(millis);
}

fn fetch_for_number(
    num_reward: u64,
    rt: &mut runtime::Runtime,
    cli: Arc<CkbAsyncClient>,
    cnt: Arc<RwLock<u64>>,
    rec: Arc<RwLock<BlockData>>,
) {
    let num_block = num_reward - constants::CONFIRMATIONS;
    let cli2 = Arc::clone(&cli);
    let fut = cli
        .block_by_number(num_block)
        .and_then(move |block| {
            let cellbase_input = block
                .transaction(0)
                .unwrap()
                .witnesses()
                .get(0)
                .unwrap()
                .raw_data();
            let address = packed::CellbaseWitnessReader::from_slice(&cellbase_input)
                .unwrap()
                .lock()
                .args()
                .raw_data()
                .to_owned();
            cli.block_hash(Some(num_reward)).map(|hash| (hash, address))
        })
        .and_then(move |(hash, address)| {
            cli2.get_cellbase_output_capacity_details(hash)
                .map(move |reward| {
                    let primary: u64 = reward.primary.into();
                    (address, primary)
                })
        })
        .then(move |result| {
            match result {
                Ok((address, reward)) => {
                    log::trace!(
                        "        block {}: reward {}, address: {}",
                        num_block,
                        reward,
                        faster_hex::hex_string(&address).unwrap(),
                    );
                    rec.write().insert(num_block, (address, reward));
                }
                Err(err) => {
                    log::trace!("    error for block {}: {:?}", num_block, err);
                }
            };
            {
                *cnt.write() += 1;
            }
            Ok(())
        });
    rt.spawn(fut);
}

pub fn fetch(args: &arguments::Arguments) -> Result<ChainData> {
    let records = Arc::new(RwLock::new(HashMap::new()));
    let count = Arc::new(RwLock::new(0));

    let mut rt = runtime::Builder::new().blocking_threads(4).build().unwrap();
    let cli_async = Arc::new(CkbAsyncClient::new(args.url().to_owned()));
    let cli_sync = CkbSyncClient::new(args.url().to_owned());

    let mut number_start = 1 + constants::CONFIRMATIONS;
    let batch_size = 512;

    log::info!(
        "waiting the specified epoch {} and syncing chain data ...",
        args.epoch()
    );
    loop {
        let tip_header = cli_sync.tip_header().expect("failed to fetch tip header");
        let tip_number = tip_header.number();
        let tip_epoch = tip_header.epoch();
        if tip_epoch.number() < args.epoch()
            || (tip_epoch.number() == args.epoch()
                && tip_epoch.index() < constants::CONFIRMATIONS - 1)
        {
            let syncing_batch = if tip_number > number_start {
                let mut number_end = number_start + batch_size;
                let syncing_batch = if number_end > tip_number {
                    number_end = tip_number;
                    false
                } else {
                    true
                };
                for num in (number_start..=number_end).into_iter() {
                    let cli = Arc::clone(&cli_async);
                    let cnt = Arc::clone(&count);
                    let rec = Arc::clone(&records);
                    fetch_for_number(num, &mut rt, cli, cnt, rec);
                }
                number_start += number_end + 1;
                syncing_batch
            } else {
                false
            };
            let is_almost_finished = tip_epoch.number() < args.epoch() - 1
                || tip_epoch.length() - tip_epoch.index() > 16;

            let wait_millis = if is_almost_finished || syncing_batch {
                2000
            } else {
                60 * 1000
            };
            log::info!(
                "    number: {}, epoch: {:#}, waiting {} ms for epoch {}({}/--) ...",
                tip_number,
                tip_epoch,
                wait_millis,
                args.epoch(),
                constants::CONFIRMATIONS - 1,
            );
            sleep_millis(wait_millis);
        } else {
            log::info!(
                "done: expect epoch {}({}/--), and current is {:#}",
                args.epoch(),
                constants::CONFIRMATIONS - 1,
                tip_epoch,
            );
            break;
        }
    }

    log::info!("epoch is reached, only syncing chain data ...");
    let number_last = {
        let tmp: u64 = cli_sync
            .epoch_by_number(args.epoch())
            .expect("failed to fetch tip header")
            .start_number
            .into();
        tmp - 1
    };
    let number_stop = number_last + constants::CONFIRMATIONS;
    loop {
        if number_stop >= number_start {
            let mut number_end = number_start + batch_size;
            if number_end > number_stop {
                number_end = number_stop;
            }
            for num in (number_start..=number_end).into_iter() {
                let cli = Arc::clone(&cli_async);
                let cnt = Arc::clone(&count);
                let rec = Arc::clone(&records);
                fetch_for_number(num, &mut rt, cli, cnt, rec);
            }
            number_start = number_end + 1;
            sleep_millis(2000);
        } else {
            break;
        }
    }

    log::info!("syncing is done, waiting for all results ...");
    loop {
        let count = { *count.read() };
        log::trace!("    will stop at {}, current {}", number_last, count);
        if count == number_last {
            let data_count = { records.read().len() } as u64;
            if count > data_count {
                log::warn!("    require {} records, but only get {}", count, data_count);
            }
            break;
        } else if count > number_last {
            return Err(Error::Unreachable);
        }
        sleep_millis(500);
    }

    log::info!("aggregate round 5.3 mined ...");
    let mut rewards = HashMap::new();
    {
        let records = records.read();
        for idx in 1..=number_last {
            let (address, reward) = records.get(&idx).cloned().unwrap_or_else(|| {
                cli_sync
                    .block_by_number(idx)
                    .map(|block| {
                        let cellbase_input = block
                            .transaction(0)
                            .unwrap()
                            .witnesses()
                            .get(0)
                            .unwrap()
                            .raw_data();
                        packed::CellbaseWitnessReader::from_slice(&cellbase_input)
                            .unwrap()
                            .lock()
                            .args()
                            .raw_data()
                            .to_owned()
                    })
                    .and_then(|address| {
                        cli_sync
                            .block_hash(Some(idx + constants::CONFIRMATIONS))
                            .map(move |hash| (hash, address))
                    })
                    .and_then(|(hash, address)| {
                        cli_sync
                            .get_cellbase_output_capacity_details(hash)
                            .map(move |reward| {
                                let primary: u64 = reward.primary.into();
                                (address, primary)
                            })
                    })
                    .unwrap()
            });
            if log::log_enabled!(log::Level::Trace) {
                log::trace!(
                    "        block {}: {}, {}",
                    idx,
                    faster_hex::hex_string(&address).unwrap(),
                    reward
                );
            }
            {
                let total_reward = rewards.entry(address).or_insert(0);
                *total_reward += reward;
            }
        }
        if log::log_enabled!(log::Level::Trace) {
            for (address, reward) in rewards.iter() {
                log::trace!(
                    "        address {}: {}",
                    faster_hex::hex_string(&address).unwrap(),
                    reward
                );
            }
        }
    }

    let header = cli_sync.header_by_number(number_last).unwrap();

    let diff_avg = (1..=constants::EPOCH_AVG_COUNT)
        .map(|i| {
            let target = cli_sync
                .epoch_by_number(args.epoch() - i)
                .unwrap()
                .compact_target;
            utilities::compact_to_difficulty(target.into())
        })
        .sum::<U256>()
        / U256::from(constants::EPOCH_AVG_COUNT);

    Ok(ChainData {
        rewards,
        header,
        diff_avg,
    })
}
