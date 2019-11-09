// Copyright (C) 2019 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{convert::TryFrom, fs, path};

use parking_lot::RwLock;
use property::Property;

use uckb_jsonrpc_client::url;

use crate::{
    constants,
    error::{Error, Result},
};

#[derive(Property)]
pub struct Arguments {
    url: url::Url,
    epoch: u64,
    output: RwLock<fs::File>,
}

pub fn build_commandline() -> Result<Arguments> {
    let yaml = clap::load_yaml!("cli.yaml");
    let matches = clap::App::from_yaml(yaml).get_matches();
    Arguments::try_from(&matches)
}

impl<'a> TryFrom<&'a clap::ArgMatches<'a>> for Arguments {
    type Error = Error;
    fn try_from(matches: &'a clap::ArgMatches) -> Result<Self> {
        let url = matches
            .value_of("url")
            .map(|url_str| url::Url::parse(url_str))
            .transpose()?
            .ok_or_else(|| Error::Unreachable("no argument 'url'".to_owned()))?;
        let epoch = matches
            .value_of("epoch")
            .map(|num_str| num_str.parse::<u64>().map(|num| num + 1))
            .transpose()?
            .ok_or_else(|| Error::Unreachable("no argument 'epoch'".to_owned()))
            .and_then(|epoch| {
                if epoch < constants::EPOCH_AVG_COUNT {
                    Err(Error::EpochTooSmall(epoch, constants::EPOCH_AVG_COUNT))
                } else {
                    Ok(epoch)
                }
            })?;
        let output = matches
            .value_of("output")
            .ok_or_else(|| Error::Unreachable("no argument 'output'".to_owned()))
            .and_then(|path_str| {
                let path = path::Path::new(path_str);
                if path.exists() {
                    Err(Error::OutputExisted(path_str.to_owned()))
                } else {
                    let file = fs::OpenOptions::new().create(true).write(true).open(path)?;
                    Ok(RwLock::new(file))
                }
            })?;
        Ok(Self { url, epoch, output })
    }
}
