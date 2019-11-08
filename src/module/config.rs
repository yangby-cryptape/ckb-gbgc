// Copyright (C) 2019 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use serde_derive::Serialize;

use uckb_jsonrpc_client::interfaces::types::{core, prelude::*, H256};

#[derive(Serialize)]
pub struct Configuration {
    pub name: String,
    pub timestamp: u64,
    pub compact_target: String,
    pub message: String,
    pub cells: Vec<Cell>,
    pub genesis_epoch_length: u64,
}

#[derive(Serialize)]
pub struct Cell {
    pub capacity: u64,
    pub lock: Lock,
}

#[derive(Serialize)]
pub struct Lock {
    pub code_hash: String,
    pub args: String,
    pub hash_type: String,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            name: "ckb".to_owned(),
            timestamp: 1_573_833_600_000,
            compact_target: "0x10000000".to_owned(),
            message: format!("lina 0x{:064}", 0),
            cells: Vec::new(),
            genesis_epoch_length: 1000,
        }
    }
}

impl Configuration {
    pub fn update_target(&mut self, target: u32) -> &mut Self {
        self.compact_target = format!("{:#x}", target);
        self
    }

    pub fn append_cells(&mut self, mut cells: Vec<Cell>) -> &mut Self {
        self.cells.append(&mut cells);
        self
    }

    pub fn update_by_last_header(&mut self, header: &core::HeaderView) -> &mut Self {
        let block_hash = header.hash().unpack();
        let epoch_length = header.epoch().length();
        let timestamp: u64 = header.timestamp();
        self.update_timestamp(timestamp)
            .update_message(&block_hash)
            .update_epoch_length(epoch_length)
    }

    fn update_timestamp(&mut self, timestamp: u64) -> &mut Self {
        self.timestamp = timestamp;
        self
    }

    fn update_epoch_length(&mut self, epoch_length: u64) -> &mut Self {
        self.genesis_epoch_length = epoch_length;
        self
    }

    fn update_message(&mut self, hash: &H256) -> &mut Self {
        self.message = format!("lina {:#x}", hash);
        self
    }
}
