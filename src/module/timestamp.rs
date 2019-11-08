// Copyright (C) 2019 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub fn timestamp(year: u64, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Option<u64> {
    if year < 1970 || month == 0 || month > 12 || hour >= 24 || minute >= 60 || second >= 60 {
        return None;
    }
    let leap = year % 400 == 0 || (year % 4 == 0 && year % 100 != 0);
    let day_max = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => unreachable!(),
    };
    if day > day_max {
        return None;
    }
    let leap_years = if year < 1972 {
        0
    } else if year <= 2000 {
        (year - 1972 + 3) / 4
    } else {
        ((year - 1972 + 3) / 4) - ((year - 2000 + 99) / 100) + ((year - 2000 + 399) / 400)
    };
    let mut count_days = (year - 1970) * 365 + leap_years;
    count_days += match month {
        1 => 0,
        2 => 31,
        3 => 31 + 28,
        4 => 31 + 28 + 31,
        5 => 31 + 28 + 31 + 30,
        6 => 31 + 28 + 31 + 30 + 31,
        7 => 31 + 28 + 31 + 30 + 31 + 30,
        8 => 31 + 28 + 31 + 30 + 31 + 30 + 31,
        9 => 31 + 28 + 31 + 30 + 31 + 30 + 31 + 31,
        10 => 31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30,
        11 => 31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31,
        12 => 31 + 28 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31 + 30,
        _ => unreachable!(),
    };
    if leap && month > 2 {
        count_days += 1;
    }
    count_days += u64::from(day) - 1;
    Some(((count_days * 24 + u64::from(hour)) * 60 + u64::from(minute)) * 60 + u64::from(second))
}
