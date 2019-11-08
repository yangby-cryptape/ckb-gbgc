// Copyright (C) 2019 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[derive(Debug, Clone, Copy)]
pub struct Token(u64);

pub const BYTE_SHANNONS: u64 = 100_000_000;

impl Token {
    pub fn from_bytes(bytes: u64) -> Self {
        Self(bytes.checked_mul(BYTE_SHANNONS).unwrap())
    }

    pub fn from_shannons(shannons: u64) -> Self {
        Self(shannons)
    }

    pub fn shannons(self) -> u64 {
        self.0
    }
}
