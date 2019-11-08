// Copyright (C) 2019 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::io::Write;

use crate::{arguments, data, error::Result, module::config};

pub fn fill(args: &arguments::Arguments, cfg: &config::Configuration) -> Result<()> {
    let tag = "specs";
    let mut tt = tinytemplate::TinyTemplate::new();
    tt.add_template(tag, data::SPECS_TEMPLATE).unwrap();
    let rendered = tt.render(tag, cfg).unwrap();
    let spec: ckb_chain_spec::ChainSpec = toml::from_slice(&rendered.as_bytes())?;
    let hash = spec.build_genesis().unwrap().hash();
    log::info!("Genesis Hash: {:#x}", hash);
    {
        let mut file = args.output().write();
        file.write_all(rendered.as_str().as_bytes())?;
        file.sync_all()?;
    }
    Ok(())
}
