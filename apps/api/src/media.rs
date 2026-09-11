//! Provider-independent media identifiers and coordinates.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A validated positive identifier from The Movie Database (`TMDb`).
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

/// Returned when a `TMDb` identifier is zero, negative, or not an integer.
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

/// A positive episode coordinate, including episodes in season zero.
#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "i32", into = "i32")]
pub struct EpisodeNumber(i32);

impl EpisodeNumber {
    #[must_use]
    pub const fn get(self) -> i32 {
        self.0
    }
}
impl TryFrom<i32> for EpisodeNumber {
    type Error = InvalidEpisodeNumber;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value > 0 {
            Ok(Self(value))
        } else {
            Err(InvalidEpisodeNumber)
        }
    }
}
impl From<EpisodeNumber> for i32 {
    fn from(value: EpisodeNumber) -> Self {
        value.0
    }
}
impl fmt::Display for EpisodeNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
#[derive(Debug, Clone, Copy, Error)]
#[error("episode number must be positive")]
pub struct InvalidEpisodeNumber;

/// An `IMDb` title identifier. This validates syntax, not a cross-provider mapping.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ImdbTitleId(String);

impl ImdbTitleId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ImdbTitleId {
    type Error = InvalidImdbTitleId;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value
            .strip_prefix("tt")
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        {
            Ok(Self(value))
        } else {
            Err(InvalidImdbTitleId)
        }
    }
}
impl FromStr for ImdbTitleId {
    type Err = InvalidImdbTitleId;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.to_owned().try_into()
    }
}
impl From<ImdbTitleId> for String {
    fn from(value: ImdbTitleId) -> Self {
        value.0
    }
}
impl fmt::Display for ImdbTitleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
#[derive(Debug, Clone, Copy, Error)]
#[error("invalid IMDb title identifier")]
pub struct InvalidImdbTitleId;

/// A catalogue entity's canonical identity, independent of language and availability.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum CatalogueId {
    Movie(TmdbId),
    Series(TmdbId),
    Season(TmdbId),
    Episode(TmdbId),
}
impl fmt::Display for CatalogueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, id) = match self {
            Self::Movie(id) => ("movie", id),
            Self::Series(id) => ("series", id),
            Self::Season(id) => ("season", id),
            Self::Episode(id) => ("episode", id),
        };
        write!(f, "tmdb:{kind}:{id}")
    }
}
impl FromStr for CatalogueId {
    type Err = InvalidCatalogueId;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split(':');
        if parts.next() != Some("tmdb") {
            return Err(InvalidCatalogueId);
        }
        let kind = parts.next().ok_or(InvalidCatalogueId)?;
        let id = parts
            .next()
            .ok_or(InvalidCatalogueId)?
            .parse()
            .map_err(|_| InvalidCatalogueId)?;
        let id = match kind {
            "movie" => Self::Movie(id),
            "series" => Self::Series(id),
            "season" => Self::Season(id),
            "episode" => Self::Episode(id),
            _ => return Err(InvalidCatalogueId),
        };
        if parts.next().is_some() || id.to_string() != value {
            return Err(InvalidCatalogueId);
        }
        Ok(id)
    }
}
impl TryFrom<String> for CatalogueId {
    type Error = InvalidCatalogueId;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl From<CatalogueId> for String {
    fn from(value: CatalogueId) -> Self {
        value.to_string()
    }
}
#[derive(Debug, Clone, Copy, Error)]
#[error("invalid catalogue identifier")]
pub struct InvalidCatalogueId;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_ids_round_trip_and_reject_noncanonical_spellings() {
        for value in [
            "tmdb:movie:12",
            "tmdb:series:12",
            "tmdb:season:12",
            "tmdb:episode:12",
        ] {
            let id: CatalogueId = value.parse().unwrap();
            assert_eq!(id.to_string(), value);
            let json = serde_json::to_string(&id).unwrap();
            assert_eq!(serde_json::from_str::<CatalogueId>(&json).unwrap(), id);
        }
        for value in [
            "imdb:movie:12",
            "tmdb:movie:0",
            "tmdb:movie:-1",
            "tmdb:movie:012",
            "tmdb:movie:+12",
            "tmdb:movie:12:extra",
            "tmdb:unknown:12",
        ] {
            assert!(value.parse::<CatalogueId>().is_err(), "{value}");
        }
        assert_ne!(
            CatalogueId::Movie(TmdbId::new(12).unwrap()),
            CatalogueId::Series(TmdbId::new(12).unwrap())
        );
    }

    #[test]
    fn validates_episode_and_imdb_coordinates_at_deserialization() {
        assert_eq!(serde_json::from_str::<EpisodeNumber>("1").unwrap().get(), 1);
        for value in ["-1", "0"] {
            assert!(serde_json::from_str::<EpisodeNumber>(value).is_err());
        }
        assert_eq!(
            "tt0123456".parse::<ImdbTitleId>().unwrap().as_str(),
            "tt0123456"
        );
        for value in ["tt", "nm1234", "tt12x", " tt123"] {
            assert!(value.parse::<ImdbTitleId>().is_err());
        }
    }

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
