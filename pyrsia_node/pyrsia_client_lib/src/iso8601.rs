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
