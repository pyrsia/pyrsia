/*
   Copyright 2021 JFrog Ltd

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/
use log::warn;
use time::format_description::FormatItem;
use time::{format_description, OffsetDateTime};

const ISO8601_FORMAT: &str = "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z";

fn iso8601_format_spec() -> Vec<FormatItem<'static>> {
    format_description::parse(ISO8601_FORMAT).unwrap() // Call unwrap because this format spec is tested and should never fail. If it does, there is nothing to do but panic.
}

pub fn as_utc_iso8601_string(t: OffsetDateTime) -> String {
    t.format(&iso8601_format_spec()).unwrap() // If the formatting fails there is no reasonable action but panic.
}

/// Return the current time formatted as a UTC ISO8601 string
pub fn now_as_utc_iso8601_string() -> String {
    as_utc_iso8601_string(OffsetDateTime::now_utc())
}

pub fn parse_iso8601(dt_string: &str) -> Option<OffsetDateTime> {
    match iso8601::datetime(dt_string) {
        Ok(date_time) => {
            match date_time.date {
                iso8601::Date::YMD { year, month, day } => Some(
                    iso8601_date_time_to_offset_date_time(year, month, day, date_time.time),
                ),
                iso8601::Date::Week {
                    year: _,
                    ww: _,
                    d: _,
                } => {
                    warn!(
                        "Unsupported timestamp in year-week-day format {}",
                        dt_string
                    );
                    None
                }
                iso8601::Date::Ordinal { year: _, ddd: _ } => {
                    warn!("Unsupported timestamp in year-day format {}", dt_string);
                    None
                }
            }
        }
        Err(error) => {
            warn!("Error parsing JSON timestamp {}", error);
            None
        }
    }
}

fn iso8601_date_time_to_offset_date_time(
    year: i32,
    month: u32,
    day: u32,
    time: iso8601::Time,
) -> OffsetDateTime {
    let formatted_date_time = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}:{:03} {:02}:{:02}",
        year,
        month,
        day,
        time.hour,
        time.minute,
        time.second,
        time.millisecond,
        time.tz_offset_hours,
        time.tz_offset_minutes
    );
    println!("{}", formatted_date_time);
    let format = time::format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second]:[subsecond] [offset_hour]:[offset_minute]",
    )
    .unwrap();
    OffsetDateTime::parse(&formatted_date_time, &format).unwrap()
}
