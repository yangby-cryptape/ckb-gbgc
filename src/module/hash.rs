// Copyright (C) 2019 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::mem;

use bech32::FromBase32;

use crate::error::{Error, Result};

pub type H160 = [u8; 20];

pub mod deprecated {
    use super::*;

    pub fn extract_from_address(address: &str) -> Option<Result<H160>> {
        bech32::decode(address)
            .ok()
            .and_then(|(ref hrp, ref base32)| {
                if hrp != "ckt" {
                    Some(Err(Error::Unreachable(format!("hrp('{}') != 'ckt'", hrp))))
                } else {
                    Vec::<u8>::from_base32(base32).ok().and_then(|bytes| {
                        if bytes.len() != 25 {
                            None
                        } else if bytes[0] != 0x01 {
                            Some(Err(Error::Unimplemented("type != bin-idx".to_owned())))
                        } else if &bytes[1..5] != b"P2PH" {
                            Some(Err(Error::Unimplemented("bin-idx != P2PH".to_owned())))
                        } else {
                            let mut hash: H160 =
                                unsafe { mem::MaybeUninit::uninit().assume_init() };
                            hash.copy_from_slice(&bytes[5..]);
                            Some(Ok(hash))
                        }
                    })
                }
            })
    }
}

fn extract_from_address_inner(address: &str, hrp_expected: &str) -> Option<Result<H160>> {
    bech32::decode(address)
        .ok()
        .and_then(|(ref hrp, ref base32)| {
            if hrp != hrp_expected {
                Some(Err(Error::Unreachable(format!(
                    "hrp('{}') != '{}'",
                    hrp, hrp_expected
                ))))
            } else {
                Vec::<u8>::from_base32(base32).ok().and_then(|bytes| {
                    if bytes.is_empty() {
                        None
                    } else if bytes[0] != 0x01 {
                        Some(Err(Error::Unimplemented(
                            "format type != short version".to_owned(),
                        )))
                    } else if bytes[1] != 0x00 {
                        Some(Err(Error::Unimplemented(
                            "code_hash_index != SECP256K1 + blake160".to_owned(),
                        )))
                    } else {
                        let mut hash: H160 = unsafe { mem::MaybeUninit::uninit().assume_init() };
                        hash.copy_from_slice(&bytes[2..]);
                        Some(Ok(hash))
                    }
                })
            }
        })
}

pub fn extract_from_address(address: &str) -> Option<Result<H160>> {
    extract_from_address_inner(address, "ckt")
}

pub fn extract_from_address_mainnet(address: &str) -> Option<Result<H160>> {
    extract_from_address_inner(address, "ckb")
}

pub fn extract_from_slice(slice: &[u8]) -> Option<H160> {
    if slice.len() == 20 {
        let mut hash: H160 = unsafe { mem::MaybeUninit::uninit().assume_init() };
        hash.copy_from_slice(slice);
        Some(hash)
    } else {
        None
    }
}
