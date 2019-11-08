// Copyright (C) 2019 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{cmp, fmt};

use property::Property;

use uckb_jsonrpc_client::interfaces::{blake2b, types::core};

use super::{
    config::{Cell, Lock},
    hash::H160,
    timestamp,
    token::Token,
};
use crate::{
    constants,
    error::{Error, Result},
};

#[derive(Debug, Property)]
pub struct Asset {
    owner: Owner,
    token: Token,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Owner {
    Single(H160),
    Multi {
        hashes: Vec<H160>,
        require_first_n: u8,
        threshold: u8,
        since: u64,
    },
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Asset {{ owner: {}, shannons: {} }}",
            self.owner,
            self.token.shannons(),
        )
    }
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Single(ref hash) => {
                write!(f, "Single(0x{})", faster_hex::hex_string(hash).unwrap())
            }
            Self::Multi {
                ref hashes,
                require_first_n,
                threshold,
                since,
            } => {
                write!(f, "Multi {{ ")?;
                write!(f, "n: {}, ", require_first_n)?;
                write!(f, "t: {}, ", threshold)?;
                write!(f, "s: {}, ", since)?;
                write!(f, "hashes: [")?;
                let mut is_first = true;
                for hash in &hashes[..] {
                    if is_first {
                        is_first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", faster_hex::hex_string(hash).unwrap())?;
                }
                write!(f, "] ")?;
                write!(f, "}}")
            }
        }
    }
}

impl fmt::LowerHex for Owner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "0x")?;
        }
        match self {
            Self::Single(ref hash) => write!(f, "{}", faster_hex::hex_string(hash).unwrap()),
            Self::Multi {
                ref hashes,
                require_first_n,
                threshold,
                since,
            } => {
                let mut bin = vec![0, *require_first_n, *threshold, hashes.len() as u8];
                for hash in &hashes[..] {
                    bin.extend_from_slice(&hash[..]);
                }
                let hash = blake2b::blake2b_256(&bin);
                let mut args = Vec::with_capacity(20 + 8);
                args.extend_from_slice(&hash[0..20]);
                args.extend_from_slice(&since.to_le_bytes()[..]);
                write!(f, "{}", faster_hex::hex_string(&args[..]).unwrap())
            }
        }
    }
}

impl cmp::Ord for Owner {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match (self.is_single(), other.is_single()) {
            (true, false) => cmp::Ordering::Less,
            (false, true) => cmp::Ordering::Greater,
            (true, true) => {
                let self_inner = if let Self::Single(ref inner) = self {
                    inner
                } else {
                    unreachable!()
                };
                let other_inner = if let Self::Single(ref inner) = other {
                    inner
                } else {
                    unreachable!()
                };
                self_inner.cmp(other_inner)
            }
            (false, false) => {
                let (self_keys, self_n, self_threshold, self_since) = if let Self::Multi {
                    hashes,
                    require_first_n,
                    threshold,
                    since,
                } = self
                {
                    (hashes, require_first_n, threshold, since)
                } else {
                    unreachable!()
                };
                let (other_keys, other_n, other_threshold, other_since) = if let Self::Multi {
                    hashes,
                    require_first_n,
                    threshold,
                    since,
                } = self
                {
                    (hashes, require_first_n, threshold, since)
                } else {
                    unreachable!()
                };
                if self_n != other_n {
                    self_n.cmp(other_n)
                } else if self_threshold != other_threshold {
                    self_threshold.cmp(other_threshold)
                } else if self_keys != other_keys {
                    self_keys.cmp(other_keys)
                } else {
                    self_since.cmp(other_since)
                }
            }
        }
    }
}

impl cmp::PartialOrd for Owner {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Asset {
    pub fn into_cell(self) -> Cell {
        Cell {
            capacity: self.token().shannons(),
            lock: Lock {
                code_hash: if self.owner().is_single() {
                    constants::SECP256K1_BLAKE160_SIGHASH_CODE_HASH
                } else {
                    constants::SECP256K1_BLAKE160_MULTISIG_CODE_HASH
                }
                .to_owned(),
                args: format!("{:#x}", self.owner()),
                hash_type: "type".to_owned(),
            },
        }
    }
}

impl Owner {
    pub fn new_single(hash: H160) -> Self {
        Self::Single(hash)
    }

    pub fn new_multi(
        hashes: Vec<H160>,
        require_first_n: u8,
        threshold: u8,
        since_str: &str,
        epoch: u64,
        planned_epoch: u64,
    ) -> Result<Self> {
        if hashes.len() >= usize::from(threshold) && threshold >= require_first_n {
            Ok(Self::Multi {
                hashes,
                require_first_n,
                threshold,
                since: parse_since_from_str(since_str, epoch, planned_epoch)?,
            })
        } else {
            Err(Error::InvalidMultiSignature)
        }
    }

    pub fn is_single(&self) -> bool {
        if let Owner::Single(_) = *self {
            true
        } else {
            false
        }
    }

    pub fn is_multi(&self) -> bool {
        !self.is_single()
    }

    pub fn with_bytes(self, bytes: u64) -> Asset {
        Asset {
            owner: self,
            token: Token::from_bytes(bytes),
        }
    }

    pub fn with_shannons(self, shannons: u64) -> Asset {
        Asset {
            owner: self,
            token: Token::from_shannons(shannons),
        }
    }

    pub fn with_token(self, token: Token) -> Asset {
        Asset { owner: self, token }
    }
}

fn parse_since_from_str(date: &str, epoch: u64, planned_epoch: u64) -> Result<u64> {
    let mut date_split = date.split('-');
    let year = date_split
        .next()
        .ok_or_else(|| Error::Unreachable(format!("split year from '{}'", date)))?
        .parse::<u64>()?;
    let month = date_split
        .next()
        .ok_or_else(|| Error::Unreachable(format!("split month from '{}'", date)))?
        .parse::<u8>()?;
    let day = date_split
        .next()
        .ok_or_else(|| Error::Unreachable(format!("split day from '{}'", date)))?
        .parse::<u8>()?;
    if date_split.next().is_some() {
        return Err(Error::Unreachable(format!(
            "'{}' has redundant fields",
            date
        )));
    }
    let start = timestamp::timestamp(2019, 11, 16, 6, 0, 0).ok_or_else(|| {
        Error::Unreachable("failed to compute timestamp for 2019-11-16 06-00-00".to_owned())
    })?;
    let end = timestamp::timestamp(year, month, day, 0, 0, 0).ok_or_else(|| {
        Error::Unreachable(format!(
            "failed to compute timestamp for {}-{}-{} 06-00-00",
            year, month, day
        ))
    })?;
    let epoches = (end - start) / (60 * 60 * 4);
    let remainder = (end - start) % (60 * 60 * 4);
    log::trace!(
        "2019-11-16 06:00:00 ({}) ~ {:04}-{:02}-{:02} 00:00:00 ({}), {}, {}",
        start,
        year,
        month,
        day,
        end,
        epoches,
        remainder
    );
    let target_epoch = {
        let (number, index) = if epoches + planned_epoch > epoch {
            (
                epoches + planned_epoch - epoch,
                remainder * 1800 / (60 * 60 * 4),
            )
        } else {
            (0, 0)
        };
        let length = 1800;
        core::EpochNumberWithFraction::new(number, index, length)
    };
    log::trace!("            target_epoch = {:#}", target_epoch);
    let since = target_epoch.full_value() | 0x2000_0000_0000_0000;
    Ok(since)
}
