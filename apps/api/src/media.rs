//! Provider-independent media identifiers and coordinates.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A validated positive identifier from The Movie Database (TMDb).
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "i64", into = "i64")]
pub struct TmdbId(i64);

impl TmdbId {
    /// Creates an identifier when `value` is positive.
    #[must_use]
    pub const fn new(value: i64) -> Option<Self> {
        if value > 0 { Some(Self(value)) } else { None }
    }

    /// Returns the numeric provider identifier.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl fmt::Display for TmdbId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<TmdbId> for i64 {
    fn from(value: TmdbId) -> Self {
        value.get()
    }
}

impl TryFrom<i64> for TmdbId {
    type Error = InvalidTmdbId;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(InvalidTmdbId)
    }
}

impl FromStr for TmdbId {
    type Err = InvalidTmdbId;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().ok().and_then(Self::new).ok_or(InvalidTmdbId)
    }
}

/// Returned when a TMDb identifier is zero, negative, or not an integer.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
#[error("TMDb ID must be a positive integer")]
pub struct InvalidTmdbId;

/// A validated, non-negative season coordinate. Zero denotes specials.
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "i32", into = "i32")]
pub struct SeasonNumber(i32);

impl SeasonNumber {
    /// Creates a season coordinate when `value` is non-negative.
    #[must_use]
    pub const fn new(value: i32) -> Option<Self> {
        if value >= 0 { Some(Self(value)) } else { None }
    }

    /// Returns the numeric season coordinate.
    #[must_use]
    pub const fn get(self) -> i32 {
        self.0
    }

    /// Reports whether this is a regular season rather than specials.
    #[must_use]
    pub const fn is_regular(self) -> bool {
        self.0 > 0
    }
}

impl fmt::Display for SeasonNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl From<SeasonNumber> for i32 {
    fn from(value: SeasonNumber) -> Self {
        value.get()
    }
}

impl TryFrom<i32> for SeasonNumber {
    type Error = InvalidSeasonNumber;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(InvalidSeasonNumber)
    }
}

impl FromStr for SeasonNumber {
    type Err = InvalidSeasonNumber;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse()
            .ok()
            .and_then(Self::new)
            .ok_or(InvalidSeasonNumber)
    }
}

/// Returned when a season coordinate is negative or not an integer.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
#[error("season number must be a non-negative integer")]
pub struct InvalidSeasonNumber;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tmdb_id_rejects_non_positive_values() {
        let valid_id = TmdbId::new(42).expect("42 is positive");

        assert_eq!(TmdbId::new(0), None);
        assert_eq!(TmdbId::new(-1), None);
        assert_eq!("42".parse::<TmdbId>().map(TmdbId::get), Ok(42));
        assert_eq!(
            serde_json::to_string(&valid_id).expect("serialize valid TMDb ID"),
            "42"
        );
        assert!(serde_json::from_str::<TmdbId>("-1").is_err());
    }

    #[test]
    fn season_number_allows_specials_but_rejects_negative_values() {
        assert_eq!(SeasonNumber::new(-1), None);
        assert!(SeasonNumber::new(0).is_some_and(|number| !number.is_regular()));
        assert!(SeasonNumber::new(1).is_some_and(SeasonNumber::is_regular));
        assert_eq!(
            serde_json::from_str::<SeasonNumber>("0")
                .expect("deserialize specials season")
                .get(),
            0
        );
    }
}
