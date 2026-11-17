//! Timestamps are naive UTC. Serde modules keep the former chrono wire format byte for byte,
//! the web UI, activity log stream and stored activity log metadata depend on it.

use std::fmt::{self, Write};

use serde::{Deserialize, Deserializer, Serializer};
use time::{
    Date, OffsetDateTime, PrimitiveDateTime, UtcOffset,
    format_description::{BorrowedFormatItem, well_known::Rfc3339},
    macros::{datetime, format_description},
};

pub const NAIVE_EPOCH: PrimitiveDateTime = datetime!(1970-01-01 0:00);

const NAIVE_ISO_PARSE_FORMAT: &[BorrowedFormatItem<'static>] = format_description!(
    "[year]-[month]-[day]T[hour]:[minute]:[second][optional [.[subsecond digits:1+]]]"
);
const DATE_FORMAT: &[BorrowedFormatItem<'static>] = format_description!("[year]-[month]-[day]");

#[must_use]
pub fn now_naive_utc() -> PrimitiveDateTime {
    OffsetDateTime::now_utc().to_naive_utc()
}

#[must_use]
pub fn naive_from_unix_timestamp(seconds: i64, nanoseconds: u32) -> Option<PrimitiveDateTime> {
    OffsetDateTime::from_unix_timestamp(seconds)
        .ok()?
        .replace_nanosecond(nanoseconds)
        .ok()
        .map(OffsetDateTimeExt::to_naive_utc)
}

pub trait OffsetDateTimeExt {
    fn to_naive_utc(self) -> PrimitiveDateTime;
}

impl OffsetDateTimeExt for OffsetDateTime {
    fn to_naive_utc(self) -> PrimitiveDateTime {
        let utc = self.to_offset(UtcOffset::UTC);
        PrimitiveDateTime::new(utc.date(), utc.time())
    }
}

/// Same output as chrono NaiveDateTime Display, e.g. 2024-03-05 13:02:03.120
#[must_use]
pub fn format_naive(datetime: PrimitiveDateTime) -> String {
    let mut out = String::with_capacity(29);
    write_naive(&mut out, datetime, ' ').expect("writing to a String cannot fail");
    out
}

/// Same output as chrono NaiveDateTime serde, e.g. 2024-03-05T13:02:03.120
#[must_use]
pub fn format_naive_iso(datetime: PrimitiveDateTime) -> String {
    let mut out = String::with_capacity(29);
    write_naive(&mut out, datetime, 'T').expect("writing to a String cannot fail");
    out
}

// chrono pads a non-zero fraction to 3, 6 or 9 digits and omits a zero one
fn write_naive(out: &mut String, datetime: PrimitiveDateTime, separator: char) -> fmt::Result {
    let year = datetime.year();
    if (0..=9999).contains(&year) {
        write!(out, "{year:04}")?;
    } else {
        write!(out, "{year:+05}")?;
    }
    write!(
        out,
        "-{:02}-{:02}{separator}{:02}:{:02}:{:02}",
        u8::from(datetime.month()),
        datetime.day(),
        datetime.hour(),
        datetime.minute(),
        datetime.second(),
    )?;
    let nanos = datetime.nanosecond();
    if nanos == 0 {
        Ok(())
    } else if nanos.is_multiple_of(1_000_000) {
        write!(out, ".{:03}", nanos / 1_000_000)
    } else if nanos.is_multiple_of(1_000) {
        write!(out, ".{:06}", nanos / 1_000)
    } else {
        write!(out, ".{nanos:09}")
    }
}

fn parse_naive_iso<E: serde::de::Error>(value: &str) -> Result<PrimitiveDateTime, E> {
    PrimitiveDateTime::parse(value, NAIVE_ISO_PARSE_FORMAT).map_err(E::custom)
}

fn parse_utc<E: serde::de::Error>(value: &str) -> Result<OffsetDateTime, E> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map(|datetime| datetime.to_offset(UtcOffset::UTC))
        .map_err(E::custom)
}

/// Same output as chrono DateTime<Utc> serde, e.g. 2024-03-05T13:02:03.120Z
#[must_use]
pub fn format_utc(datetime: OffsetDateTime) -> String {
    let mut out = format_naive_iso(datetime.to_naive_utc());
    out.push('Z');
    out
}

pub mod serde_naive {
    use super::{Deserialize, Deserializer, PrimitiveDateTime, Serializer};

    pub fn serialize<S: Serializer>(
        datetime: &PrimitiveDateTime,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&super::format_naive_iso(*datetime))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<PrimitiveDateTime, D::Error> {
        super::parse_naive_iso(&String::deserialize(deserializer)?)
    }

    pub mod option {
        use super::{Deserialize, Deserializer, PrimitiveDateTime, Serializer};

        pub fn serialize<S: Serializer>(
            datetime: &Option<PrimitiveDateTime>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            match datetime {
                Some(datetime) => super::serialize(datetime, serializer),
                None => serializer.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Option<PrimitiveDateTime>, D::Error> {
            Option::<String>::deserialize(deserializer)?
                .map(|value| crate::time_utils::parse_naive_iso(&value))
                .transpose()
        }
    }
}

/// Input accepts any RFC 3339 offset and is normalized to UTC like chrono DateTime<Utc>.
pub mod serde_utc {
    use super::{Deserialize, Deserializer, OffsetDateTime, Serializer};

    pub fn serialize<S: Serializer>(
        datetime: &OffsetDateTime,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&super::format_utc(*datetime))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<OffsetDateTime, D::Error> {
        super::parse_utc(&String::deserialize(deserializer)?)
    }

    pub mod option {
        use super::{Deserialize, Deserializer, OffsetDateTime, Serializer};

        pub fn serialize<S: Serializer>(
            datetime: &Option<OffsetDateTime>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            match datetime {
                Some(datetime) => super::serialize(datetime, serializer),
                None => serializer.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Option<OffsetDateTime>, D::Error> {
            Option::<String>::deserialize(deserializer)?
                .map(|value| crate::time_utils::parse_utc(&value))
                .transpose()
        }
    }
}

pub mod serde_date {
    use serde::de::Error as _;

    use super::{DATE_FORMAT, Date, Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(date: &Date, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(
            &date
                .format(DATE_FORMAT)
                .map_err(serde::ser::Error::custom)?,
        )
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Date, D::Error> {
        Date::parse(&String::deserialize(deserializer)?, DATE_FORMAT).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use time::macros::{date, datetime};

    use super::*;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Naive {
        #[serde(with = "serde_naive")]
        at: PrimitiveDateTime,
        #[serde(default, with = "serde_naive::option")]
        maybe: Option<PrimitiveDateTime>,
    }

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Utc {
        #[serde(with = "serde_utc")]
        at: OffsetDateTime,
    }

    // expected strings were produced by chrono 0.4.45
    #[test]
    fn naive_matches_chrono_format() {
        let cases = [
            (
                datetime!(2024-03-05 01:02:03),
                "2024-03-05T01:02:03",
                "2024-03-05 01:02:03",
            ),
            (
                datetime!(2024-03-05 13:02:03.120),
                "2024-03-05T13:02:03.120",
                "2024-03-05 13:02:03.120",
            ),
            (
                datetime!(2024-03-05 13:02:03.123456),
                "2024-03-05T13:02:03.123456",
                "2024-03-05 13:02:03.123456",
            ),
            (
                datetime!(2024-03-05 23:59:59.123456789),
                "2024-03-05T23:59:59.123456789",
                "2024-03-05 23:59:59.123456789",
            ),
            (
                datetime!(2024-03-05 0:00:00.0001),
                "2024-03-05T00:00:00.000100",
                "2024-03-05 00:00:00.000100",
            ),
        ];
        for (datetime, iso, display) in cases {
            assert_eq!(format_naive_iso(datetime), iso);
            assert_eq!(format_naive(datetime), display);
            let value = Naive {
                at: datetime,
                maybe: Some(datetime),
            };
            let json = serde_json::to_string(&value).unwrap();
            assert_eq!(json, format!(r#"{{"at":"{iso}","maybe":"{iso}"}}"#));
            assert_eq!(serde_json::from_str::<Naive>(&json).unwrap(), value);
        }
    }

    #[test]
    fn naive_option_and_parsing() {
        let value: Naive = serde_json::from_str(r#"{"at":"2023-05-05T23:56:04"}"#).unwrap();
        assert_eq!(value.at, datetime!(2023-05-05 23:56:04));
        assert_eq!(value.maybe, None);
        assert_eq!(
            serde_json::to_string(&Naive {
                at: value.at,
                maybe: None
            })
            .unwrap(),
            r#"{"at":"2023-05-05T23:56:04","maybe":null}"#
        );
        assert!(serde_json::from_str::<Naive>(r#"{"at":"2024-03-05 01:02:03"}"#).is_err());
        assert!(serde_json::from_str::<Naive>(r#"{"at":"2024-03-05T01:02"}"#).is_err());
    }

    #[test]
    fn utc_matches_chrono_format() {
        let value = Utc {
            at: datetime!(2024-03-05 13:02:03.120 UTC),
        };
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, r#"{"at":"2024-03-05T13:02:03.120Z"}"#);
        assert_eq!(serde_json::from_str::<Utc>(&json).unwrap(), value);

        let shifted: Utc = serde_json::from_str(r#"{"at":"2024-03-05T01:02:03+02:00"}"#).unwrap();
        assert_eq!(shifted.at, datetime!(2024-03-04 23:02:03 UTC));
        assert_eq!(shifted.at.offset(), UtcOffset::UTC);
    }

    #[test]
    fn date_roundtrip() {
        #[derive(Debug, Deserialize, PartialEq, Serialize)]
        struct Release {
            #[serde(with = "serde_date")]
            date: Date,
        }
        let json = r#"{"date":"2024-03-05"}"#;
        let value: Release = serde_json::from_str(json).unwrap();
        assert_eq!(value.date, date!(2024 - 03 - 05));
        assert_eq!(serde_json::to_string(&value).unwrap(), json);
    }

    #[test]
    fn unix_timestamp_conversion() {
        assert_eq!(
            naive_from_unix_timestamp(1_709_643_723, 120_000_000),
            Some(datetime!(2024-03-05 13:02:03.120))
        );
        assert_eq!(NAIVE_EPOCH, naive_from_unix_timestamp(0, 0).unwrap());
    }
}
