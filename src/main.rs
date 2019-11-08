// Copyright (C) 2019 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub mod arguments;
pub mod client;
pub mod constants;
pub mod data;
pub mod error;
pub mod module;
pub mod preprocess;
pub mod template;

fn execute() -> error::Result<()> {
    let args = arguments::build_commandline()?;
    let chain_data = client::fetch(&args)?;
    let mut cfg = module::config::Configuration::default();
    cfg.update_by_last_header(chain_data.header());
    let (cells, target) = preprocess::process(&args, &chain_data, &cfg)?;
    cfg.append_cells(cells).update_target(target);
    template::fill(&args, &cfg)
}

fn main() {
    pretty_env_logger::init();

    if let Err(error) = execute() {
        eprintln!("Fatal: {}", error);
        ::std::process::exit(1);
    }
}
