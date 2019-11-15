// Copyright (C) 2019 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::HashMap;

use uckb_jsonrpc_client::interfaces::types::{prelude::Unpack, utilities, U256};

use crate::{
    arguments, client, constants, data,
    error::{Error, Result},
    module::{asset, config, hash, token},
};

pub fn process(
    args: &arguments::Arguments,
    chain_data: &client::ChainData,
    cfg: &config::Configuration,
) -> Result<(Vec<config::Cell>, u32)> {
    let mut cells = Vec::new();

    // Satoshi Gift
    let satoshi_cell = config::Cell {
        capacity: constants::INITIAL_TOTAL_SUPPLY / 4,
        lock: config::Lock {
            code_hash: constants::SATOSHI_GIFT_CODE_HASH.to_owned(),
            args: constants::SATOSHI_GIFT_ARGS.to_owned(),
            hash_type: "data".to_owned(),
        },
    };
    log::info!("burned part = {}", satoshi_cell.capacity);
    cells.push(satoshi_cell);

    // Imported Part
    {
        let imported_expected = (u128::from(constants::INITIAL_TOTAL_SUPPLY) * 725 / 1000) as u64;
        let mut imported_cells = data::GENESIS_ALLOCATE
            .lines()
            .map(|line| {
                let mut part = line.split(',');
                let addr = part
                    .next()
                    .ok_or_else(|| Error::Unreachable(format!("split address from '{}'", line)))?;
                let ckb = part
                    .next()
                    .ok_or_else(|| Error::Unreachable(format!("split ckb from '{}'", line)))?
                    .parse::<u64>()?;
                let date_opt = part.next();
                if part.next().is_some() {
                    Err(Error::Unreachable(format!(
                        "'{}' has redundant fileds",
                        line
                    )))
                } else {
                    let hash = hash::extract_from_address_mainnet(addr)
                        .transpose()?
                        .ok_or_else(|| {
                            Error::Unreachable(format!("parse mainnet address from '{}'", addr))
                        })?;
                    let cell = if let Some(date) = date_opt {
                        if date == "" || date == "\"\"" {
                            asset::Owner::new_single(hash).with_bytes(ckb)
                        } else {
                            asset::Owner::new_multi(vec![hash], 0, 1, date, args.epoch())
                                .map(|owner| owner.with_bytes(ckb))?
                        }
                    } else {
                        asset::Owner::new_single(hash).with_bytes(ckb)
                    }
                    .into_cell();
                    Ok(cell)
                }
            })
            .collect::<Result<Vec<_>>>()?;
        let imported_actual = imported_cells.iter().map(|cell| cell.capacity).sum::<u64>();
        log::info!("imported part = {}", imported_actual);
        if imported_expected != imported_actual {
            return Err(Error::Unreachable(format!(
                "imported capacity: expected: {}, actual: {}",
                imported_expected, imported_actual
            )));
        }
        cells.append(&mut imported_cells);
    }

    // Foundation Reserve
    let foundation_spent = {
        let res = ckb_resource::Resource::bundled("specs/mainnet.toml".to_owned());
        let spec = ckb_chain_spec::ChainSpec::load_from(&res).unwrap();
        let genesis_block = spec.build_genesis().unwrap().data();
        genesis_block
            .as_reader()
            .transactions()
            .get(0)
            .map(|tx| {
                tx.raw()
                    .outputs()
                    .iter()
                    .enumerate()
                    .map(|(index, output)| {
                        if index < 6 {
                            output.capacity().unpack()
                        } else {
                            0u64
                        }
                    })
                    .sum::<u64>()
                    - tx.raw().outputs_data().get_unchecked(0).raw_data().len() as u64
                        * token::BYTE_SHANNONS
                    + cfg.message.as_bytes().len() as u64 * token::BYTE_SHANNONS
            })
            .ok_or_else(|| Error::Unreachable("compute foundation spent".to_owned()))?
    };
    log::info!("foundation spent = {}", foundation_spent);
    if foundation_spent != 1_264_963 * token::BYTE_SHANNONS {
        return Err(Error::Unreachable(format!(
            "foundation_spent(={}) should be 1_264_963 * 1_0000_0000",
            foundation_spent
        )));
    }

    {
        let foundation_reserve = constants::INITIAL_TOTAL_SUPPLY * 2 / 100 - foundation_spent;

        let foundation_cell = hash::extract_from_address_mainnet(constants::FOUNDATION_ADDR)
            .ok_or_else(|| {
                Error::Unreachable(format!(
                    "parse mainnet address from '{}'",
                    constants::FOUNDATION_ADDR
                ))
            })?
            .and_then(|hash| {
                asset::Owner::new_multi(vec![hash], 0, 1, constants::FOUNDATION_SINCE, args.epoch())
                    .map(|owner| {
                        log::trace!("foundation owner = {}", owner);
                        owner.with_shannons(foundation_reserve)
                    })
            })
            .map(asset::Asset::into_cell)?;
        log::info!("foundation part = {}", foundation_cell.capacity);
        cells.push(foundation_cell);
    }

    let (assets_competition, remained, target) = process_competition(chain_data)?;
    let mut competition_cells = assets_competition
        .into_iter()
        .map(|x| x.into_cell())
        .collect::<Vec<_>>();
    cells.append(&mut competition_cells);

    // Testnet Remained
    let testnet_cell = hash::extract_from_address_mainnet(constants::FOUNDATION_TESTNET_ADDR)
        .ok_or_else(|| {
            Error::Unreachable(format!(
                "parse mainnet address from '{}'",
                constants::FOUNDATION_TESTNET_ADDR
            ))
        })?
        .map(|hash| {
            asset::Owner::new_single(hash)
                .with_shannons(remained)
                .into_cell()
        })?;
    log::info!("foundation testnet part = {}", testnet_cell.capacity);
    cells.push(testnet_cell);

    let total_supply = cells.iter().map(|cell| cell.capacity).sum::<u64>() + foundation_spent;
    if total_supply != constants::INITIAL_TOTAL_SUPPLY {
        return Err(Error::Unreachable(format!(
            "total supply: expected: {}, actual: {}",
            constants::INITIAL_TOTAL_SUPPLY,
            total_supply
        )));
    }

    Ok((cells, target))
}

macro_rules! assets_append {
    ($total:ident, $part:ident, $tag:literal) => {
        let reward = $part
            .iter()
            .map(|asset| asset.token().shannons())
            .sum::<u64>();
        log::info!(
            "        total reward for {}: {} accounts, {} ckb",
            $tag,
            $part.len(),
            reward / token::BYTE_SHANNONS
        );
        if log::log_enabled!(log::Level::Debug) {
            for asset in $part.iter() {
                log::debug!("            {}", asset);
            }
        }
        $total.append(&mut $part);
    };
}

#[allow(clippy::cognitive_complexity)]
fn process_competition(chain_data: &client::ChainData) -> Result<(Vec<asset::Asset>, u64, u32)> {
    let mut expected_total_reward = 0u64;
    let target;
    let assets_total = {
        let least_token_reward = 61;
        let mut assets_total = Vec::new();
        {
            let mut assets = Vec::new();
            let mut reader = csv::Reader::from_reader(data::ROUND_1_AWARDS.as_bytes());
            for (index, result) in reader.records().enumerate() {
                let record = result?;
                if record.len() != 2 {
                    return Err(Error::Unreachable(
                        "round-1 awards record length".to_owned(),
                    ));
                }
                let hash = {
                    let addr_str = record.get(0).unwrap();
                    if let Some(hash_result) = hash::deprecated::extract_from_address(addr_str) {
                        hash_result?
                    } else {
                        continue;
                    }
                };
                let token = match index {
                    0 => token::Token::from_bytes(200_000),
                    1 => token::Token::from_bytes(100_000),
                    2 => token::Token::from_bytes(60_000),
                    _ => {
                        return Err(Error::Unreachable(
                            "round 1 awards only 3 winners".to_owned(),
                        ))
                    }
                };
                expected_total_reward += token.shannons();
                let asset = asset::Owner::new_single(hash).with_token(token);
                assets.push(asset);
            }
            assets_append!(assets_total, assets, "round 1 awards");
        }
        {
            let mut assets = Vec::new();
            let mut reader = csv::Reader::from_reader(data::ROUND_1_LOTTERY.as_bytes());
            let mut counter = 0;
            let lottery_reward = 10_000;
            for result in reader.records() {
                let record = result?;
                if record.len() != 2 {
                    return Err(Error::Unreachable(
                        "round-1 lottery record length".to_owned(),
                    ));
                }
                counter += 1;
                let hash = {
                    let addr_str = record.get(0).unwrap();
                    if let Some(hash_result) = hash::deprecated::extract_from_address(addr_str) {
                        hash_result?
                    } else {
                        continue;
                    }
                };
                let asset = asset::Owner::new_single(hash).with_bytes(lottery_reward);
                assets.push(asset);
            }
            if counter != 64 {
                return Err(Error::Unreachable(
                    "round 1 lottery only 64 winners".to_owned(),
                ));
            }
            expected_total_reward += lottery_reward * 64 * token::BYTE_SHANNONS;
            assets_append!(assets_total, assets, "round 1 lottery");
        }
        {
            let mut assets = Vec::new();
            let mut reader = csv::Reader::from_reader(data::ROUND_2_MINED.as_bytes());
            let reward_pool = 2_000_000;
            let least_block_reward = 4_000;
            let mut counter = 0;
            let mut total_block_reward = 0;
            let mut total_token_reward = 0;
            let mut check_data = Vec::new();
            for result in reader.records() {
                let record = result?;
                if record.len() != 4 {
                    return Err(Error::Unreachable("round-2 mined record length".to_owned()));
                }
                counter += 1;
                let block_reward = record.get(1).unwrap().parse::<u64>().unwrap();
                if block_reward < least_block_reward {
                    return Err(Error::Unreachable(
                        "round-2 mined block_reward < least_block_reward".to_owned(),
                    ));
                }
                let token_reward = record.get(3).unwrap().parse::<u64>().unwrap();
                if token_reward < least_token_reward {
                    return Err(Error::Unreachable(
                        "round-2 mined token_reward < least_token_reward".to_owned(),
                    ));
                }
                total_block_reward += block_reward;
                total_token_reward += token_reward;
                check_data.push((block_reward, token_reward));
                let hash = {
                    let addr_str = record.get(0).unwrap();
                    if let Some(hash_result) = hash::deprecated::extract_from_address(addr_str) {
                        hash_result?
                    } else {
                        continue;
                    }
                };
                let asset = asset::Owner::new_single(hash).with_bytes(token_reward);
                assets.push(asset);
            }
            for (block_reward, token_reward) in &check_data[..] {
                let expected_token_reward = u128::from(*block_reward) * u128::from(reward_pool)
                    / u128::from(total_block_reward);
                if expected_token_reward != u128::from(*token_reward) {
                    return Err(Error::Unreachable(
                        "round-2 mined expected_token_reward != token_reward".to_owned(),
                    ));
                }
            }
            if (reward_pool - counter) > total_token_reward || total_token_reward > reward_pool {
                return Err(Error::Unreachable(
                    "round-2 mined check total_token_reward".to_owned(),
                ));
            }
            expected_total_reward += reward_pool * token::BYTE_SHANNONS;
            assets_append!(assets_total, assets, "round 2 mined");
        }
        {
            let mut assets = Vec::new();
            let mut reader = csv::Reader::from_reader(data::ROUND_2_LUCKY.as_bytes());
            let reward_pool = 2_000_000;
            let winners = 80;
            let mut counter = 0;
            for result in reader.records() {
                let record = result?;
                if record.len() != 3 {
                    return Err(Error::Unreachable("round-2 lucky record length".to_owned()));
                }
                let epoch = record.get(0).unwrap().parse::<u64>().unwrap();
                if epoch == 0 || epoch > winners {
                    continue;
                }
                counter += 1;
                let hash = {
                    let addr_str = record.get(1).unwrap();
                    if let Some(hash_result) = hash::deprecated::extract_from_address(addr_str) {
                        hash_result?
                    } else {
                        continue;
                    }
                };
                let asset = asset::Owner::new_single(hash).with_bytes(reward_pool / winners);
                assets.push(asset);
            }
            if counter != winners {
                return Err(Error::Unreachable(format!(
                    "count(={}) for winners(={}) is not match",
                    counter, winners
                )));
            }
            expected_total_reward += reward_pool * token::BYTE_SHANNONS;
            assets_append!(assets_total, assets, "round 2 lucky");
        }
        {
            let mut assets = Vec::new();
            let mut reader = csv::Reader::from_reader(data::ROUND_3_MINED.as_bytes());
            let reward_pool = 3_000_000;
            let least_block_reward = 3_000;
            let mut counter = 0;
            let mut total_block_reward = 0;
            let mut total_token_reward = 0;
            let mut check_data = Vec::new();
            for result in reader.records() {
                let record = result?;
                if record.len() != 4 {
                    return Err(Error::Unreachable("round-3 mined record length".to_owned()));
                }
                counter += 1;
                let block_reward = record.get(2).unwrap().parse::<u64>().unwrap();
                if block_reward < least_block_reward {
                    return Err(Error::Unreachable(
                        "round-3 mined block_reward < least_block_reward".to_owned(),
                    ));
                }
                let token_reward = record.get(3).unwrap().parse::<u64>().unwrap();
                if token_reward < least_token_reward {
                    return Err(Error::Unreachable(
                        "round-3 mined token_reward < least_token_reward".to_owned(),
                    ));
                }
                total_block_reward += block_reward;
                total_token_reward += token_reward;
                check_data.push((block_reward, token_reward));
                let hash = {
                    let addr_str = record.get(0).unwrap();
                    if let Some(hash_result) = hash::extract_from_address(addr_str) {
                        hash_result?
                    } else {
                        continue;
                    }
                };
                let asset = asset::Owner::new_single(hash).with_bytes(token_reward);
                assets.push(asset);
            }
            for (block_reward, token_reward) in &check_data[..] {
                let expected_token_reward = u128::from(*block_reward) * u128::from(reward_pool)
                    / u128::from(total_block_reward);
                if expected_token_reward != u128::from(*token_reward) {
                    return Err(Error::Unreachable(
                        "round-3 mined expected_token_reward != token_reward".to_owned(),
                    ));
                }
            }
            if (reward_pool - counter) > total_token_reward || total_token_reward > reward_pool {
                return Err(Error::Unreachable(
                    "round-3 mined check total_token_reward".to_owned(),
                ));
            }
            expected_total_reward += reward_pool * token::BYTE_SHANNONS;
            assets_append!(assets_total, assets, "round 3 mined");
        }
        {
            let mut assets = Vec::new();
            let mut reader = csv::Reader::from_reader(data::ROUND_3_LUCKY.as_bytes());
            let reward_pool = 3_000_000;
            let winners = 80;
            let mut counter = 0;
            for result in reader.records() {
                let record = result?;
                if record.len() != 3 {
                    return Err(Error::Unreachable("round-3 lucky record length".to_owned()));
                }
                let epoch = record.get(0).unwrap().parse::<u64>().unwrap();
                if epoch == 0 || epoch > winners {
                    continue;
                }
                counter += 1;
                let hash = {
                    let addr_str = record.get(1).unwrap();
                    if let Some(hash_result) = hash::extract_from_address(addr_str) {
                        hash_result?
                    } else {
                        continue;
                    }
                };
                let asset = asset::Owner::new_single(hash).with_bytes(reward_pool / winners);
                assets.push(asset);
            }
            if counter != winners {
                return Err(Error::Unreachable(format!(
                    "count(={}) for winners(={}) is not match",
                    counter, winners
                )));
            }
            expected_total_reward += reward_pool * token::BYTE_SHANNONS;
            assets_append!(assets_total, assets, "round 3 lucky");
        }
        {
            let mut assets = Vec::new();
            let mut reader = csv::Reader::from_reader(data::ROUND_4_MINED.as_bytes());
            let reward_pool = 9_000_000;
            let least_block_reward = 1_000;
            let mut counter = 0;
            let mut total_block_reward = 0;
            let mut total_token_reward = 0;
            let mut check_data = Vec::new();
            for result in reader.records() {
                let record = result?;
                if record.len() != 4 {
                    return Err(Error::Unreachable("round-4 mined record length".to_owned()));
                }
                counter += 1;
                let block_reward = record.get(1).unwrap().parse::<u64>().unwrap();
                if block_reward < least_block_reward {
                    return Err(Error::Unreachable(
                        "round-4 mined block_reward < least_block_reward".to_owned(),
                    ));
                }
                let token_reward = record.get(3).unwrap().parse::<u64>().unwrap();
                if token_reward < least_token_reward {
                    return Err(Error::Unreachable(
                        "round-4 mined token_reward < least_token_reward".to_owned(),
                    ));
                }
                total_block_reward += block_reward;
                total_token_reward += token_reward;
                check_data.push((block_reward, token_reward));
                let hash = {
                    let addr_str = record.get(0).unwrap();
                    if let Some(hash_result) = hash::extract_from_address(addr_str) {
                        hash_result?
                    } else {
                        continue;
                    }
                };
                let asset = asset::Owner::new_single(hash).with_bytes(token_reward);
                assets.push(asset);
            }
            for (block_reward, token_reward) in &check_data[..] {
                let expected_token_reward = u128::from(*block_reward) * u128::from(reward_pool)
                    / u128::from(total_block_reward);
                if expected_token_reward != u128::from(*token_reward) {
                    return Err(Error::Unreachable(
                        "round-4 mined expected_token_reward != token_reward".to_owned(),
                    ));
                }
            }
            if (reward_pool - counter) > total_token_reward || total_token_reward > reward_pool {
                return Err(Error::Unreachable(
                    "round-4 mined check total_token_reward".to_owned(),
                ));
            }
            expected_total_reward += reward_pool * token::BYTE_SHANNONS;
            assets_append!(assets_total, assets, "round 4 mined");
        }
        {
            let mut assets = Vec::new();
            let mut reader = csv::Reader::from_reader(data::ROUND_5_S1_MINED.as_bytes());
            let reward_pool = 12_000_000;
            let least_block_reward = 1_000;
            let mut counter = 0;
            let mut total_block_reward = 0;
            let mut total_token_reward = 0;
            let mut check_data = Vec::new();
            for result in reader.records() {
                let record = result?;
                if record.len() != 4 {
                    return Err(Error::Unreachable(
                        "round-5.1 mined record length".to_owned(),
                    ));
                }
                counter += 1;
                let block_reward = record.get(2).unwrap().parse::<u64>().unwrap();
                if block_reward < least_block_reward {
                    return Err(Error::Unreachable(
                        "round-5.1 block_reward < least_block_reward".to_owned(),
                    ));
                }
                let token_reward = record.get(3).unwrap().parse::<u64>().unwrap();
                if token_reward < least_token_reward {
                    return Err(Error::Unreachable(
                        "round-5.1 token_reward < least_token_reward".to_owned(),
                    ));
                }
                total_block_reward += block_reward;
                total_token_reward += token_reward;
                check_data.push((block_reward, token_reward));
                let hash = {
                    let addr_str = record.get(0).unwrap();
                    if let Some(hash_result) = hash::extract_from_address(addr_str) {
                        hash_result?
                    } else {
                        continue;
                    }
                };
                let asset = asset::Owner::new_single(hash).with_bytes(token_reward);
                assets.push(asset);
            }
            for (block_reward, token_reward) in &check_data[..] {
                let expected_token_reward = u128::from(*block_reward) * u128::from(reward_pool)
                    / u128::from(total_block_reward);
                if expected_token_reward != u128::from(*token_reward) {
                    return Err(Error::Unreachable(
                        "round-5.1 expected_token_reward != token_reward".to_owned(),
                    ));
                }
            }
            if (reward_pool - counter) > total_token_reward || total_token_reward > reward_pool {
                return Err(Error::Unreachable(
                    "round-5.1 check total_token_reward".to_owned(),
                ));
            }
            expected_total_reward += reward_pool * token::BYTE_SHANNONS;
            assets_append!(assets_total, assets, "round 5.1 mined");
        }
        {
            let mut assets = Vec::new();
            let mut reader = csv::Reader::from_reader(data::ROUND_5_S2_MINED.as_bytes());
            let reward_pool = 15_000_000;
            let least_block_reward = 1_000;
            let mut counter = 0;
            let mut total_block_reward = 0;
            let mut total_token_reward = 0;
            let mut check_data = Vec::new();
            for result in reader.records() {
                let record = result?;
                if record.len() != 4 {
                    return Err(Error::Unreachable(
                        "round-5.2 mined record length".to_owned(),
                    ));
                }
                counter += 1;
                let block_reward = record.get(2).unwrap().parse::<u64>().unwrap();
                if block_reward < least_block_reward {
                    return Err(Error::Unreachable(
                        "round-5.2 block_reward < least_block_reward".to_owned(),
                    ));
                }
                let token_reward = record.get(3).unwrap().parse::<u64>().unwrap();
                if token_reward < least_token_reward {
                    return Err(Error::Unreachable(
                        "round-5.2 token_reward < least_token_reward".to_owned(),
                    ));
                }
                total_block_reward += block_reward;
                total_token_reward += token_reward;
                check_data.push((block_reward, token_reward));
                let hash = {
                    let addr_str = record.get(0).unwrap();
                    if let Some(hash_result) = hash::extract_from_address(addr_str) {
                        hash_result?
                    } else {
                        continue;
                    }
                };
                let asset = asset::Owner::new_single(hash).with_bytes(token_reward);
                assets.push(asset);
            }
            for (block_reward, token_reward) in &check_data[..] {
                let expected_token_reward = u128::from(*block_reward) * u128::from(reward_pool)
                    / u128::from(total_block_reward);
                if expected_token_reward != u128::from(*token_reward) {
                    return Err(Error::Unreachable(
                        "round-5.2 expected_token_reward != token_reward".to_owned(),
                    ));
                }
            }
            if (reward_pool - counter) > total_token_reward || total_token_reward > reward_pool {
                return Err(Error::Unreachable(
                    "round-5.2 check total_token_reward".to_owned(),
                ));
            }
            expected_total_reward += reward_pool * token::BYTE_SHANNONS;
            assets_append!(assets_total, assets, "round 5.2 mined");
        }
        {
            let mut assets = Vec::new();
            let reward_pool = 18_000_000u64;
            let least_block_reward = 1_000;
            let mut counter = 0;
            let mut total_token_reward = 0;
            let r5s3_data = chain_data
                .rewards()
                .iter()
                .filter(|(_, block_reward)| **block_reward >= least_block_reward)
                .collect::<HashMap<_, _>>();
            let total_block_reward = r5s3_data.values().cloned().sum::<u64>();
            for (slice, block_reward) in r5s3_data {
                counter += 1;
                if let Some(hash) = hash::extract_from_slice(&slice) {
                    let token_reward = ((u128::from(*block_reward)
                        * u128::from(reward_pool * token::BYTE_SHANNONS)
                        / u128::from(total_block_reward))
                        as u64)
                        / token::BYTE_SHANNONS;
                    total_token_reward += token_reward;
                    let asset = asset::Owner::new_single(hash).with_bytes(token_reward);
                    assets.push(asset);
                }
            }
            if (reward_pool - counter) > total_token_reward || total_token_reward > reward_pool {
                return Err(Error::Unreachable(
                    "round-5.3 mined check total_token_reward".to_owned(),
                ));
            }
            expected_total_reward += reward_pool * token::BYTE_SHANNONS;
            assets_append!(assets_total, assets, "round 5.3 mined");

            let diff = chain_data.diff_avg() * U256::from(3u8) / U256::from(2u8)
                * U256::from(chain_data.rewards().values().sum::<u64>())
                / U256::from(reward_pool * token::BYTE_SHANNONS);
            target = utilities::difficulty_to_compact(diff);
        }
        assets_total
    };
    log::info!("    testnet assets total = {}", assets_total.len());
    let assets_ordered = {
        let mut assets_unique = HashMap::new();
        for asset in assets_total.iter() {
            let shannons = assets_unique.entry(asset.owner()).or_insert(0);
            *shannons += asset.token().shannons();
        }
        let mut assets = Vec::new();
        for (owner, shannons) in assets_unique.into_iter() {
            assets.push(owner.to_owned().with_shannons(shannons));
        }
        assets.sort_by(|ref a, ref b| a.owner().cmp(b.owner()));
        assets
    };
    let total_reward = assets_ordered
        .iter()
        .map(|asset| asset.token().shannons())
        .sum::<u64>();
    log::info!("    testnet assets unique = {}", assets_ordered.len());
    log::info!(
        "    testnet expected total reward = {}",
        expected_total_reward
    );
    log::info!("    testnet   actual total reward = {}", total_reward);
    if expected_total_reward < total_reward {
        return Err(Error::Unreachable(format!(
            "expected_total_reward(={}) < total_reward(={})",
            expected_total_reward, total_reward
        )));
    }
    let remained = constants::INITIAL_TOTAL_SUPPLY / 200 - total_reward;
    log::info!("    testnet remained tokens = {}", remained);

    Ok((assets_ordered, remained, target))
}
