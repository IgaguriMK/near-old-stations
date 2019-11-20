use chrono::{DateTime, TimeZone, Utc};
use serde::{self, Deserialize, Deserializer};

const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Deserialize::deserialize(deserializer)?;
    s.map(|s| {
        Utc.datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
    })
    .transpose()
}
