// Copyright (C) 2019 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{io, num};

use failure::Fail;

use uckb_jsonrpc_client::url;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "internal error: should be unreachable, {}", _0)]
    Unreachable(String),
    #[fail(display = "internal error: {} should be implemented", _0)]
    Unimplemented(String),

    #[fail(display = "data error: invalid multi signature")]
    InvalidMultiSignature,

    #[fail(display = "io error: {}", _0)]
    IO(io::Error),
    #[fail(display = "number error: {}", _0)]
    Num(num::ParseIntError),
    #[fail(display = "url error: {}", _0)]
    Url(url::ParseError),
    #[fail(display = "csv error: {}", _0)]
    CSV(csv::Error),
    #[fail(display = "toml error: {}", _0)]
    Toml(toml::de::Error),

    #[fail(
        display = "argument error: the epoch is too small (expected {}, actual {})",
        _1, _0
    )]
    EpochTooSmall(u64, u64),
    #[fail(display = "argument error: the path of input file ({}) is existed", _0)]
    InputNotExisted(String),
    #[fail(
        display = "argument error: the path of output file ({}) is existed",
        _0
    )]
    OutputExisted(String),
}

pub type Result<T> = ::std::result::Result<T, Error>;

macro_rules! convert_error {
    ($name:ident, $inner_error:ty) => {
        impl ::std::convert::From<$inner_error> for Error {
            fn from(error: $inner_error) -> Self {
                Self::$name(error)
            }
        }
    };
}

convert_error!(IO, io::Error);
convert_error!(Num, num::ParseIntError);
convert_error!(Url, url::ParseError);
convert_error!(CSV, csv::Error);
convert_error!(Toml, toml::de::Error);
