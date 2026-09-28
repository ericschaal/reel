use super::Error;
use serde::{Deserialize, Serialize};

/// Finite, non-negative seconds represented as Jellyfin's signed tick count.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(try_from = "f64")]
pub(super) struct PlaybackPosition(i64);
impl TryFrom<f64> for PlaybackPosition {
    type Error = Error;
    fn try_from(seconds: f64) -> Result<Self, Error> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(Error::InvalidStartPosition);
        }
        let ticks = (seconds * 10_000_000.0)
            .round()
            .to_string()
            .parse()
            .map_err(|_| Error::InvalidStartPosition)?;
        Ok(Self(ticks))
    }
}
impl PlaybackPosition {
    pub(super) fn jellyfin_ticks(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(try_from = "i32", into = "i32")]
pub(super) struct AudioStreamIndex(i32);
#[derive(Debug, Clone, Copy)]
pub(super) struct SubtitleStreamIndex(i32);
impl TryFrom<i32> for AudioStreamIndex {
    type Error = &'static str;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(Self(value))
        } else {
            Err("audio stream index must be non-negative")
        }
    }
}
impl From<AudioStreamIndex> for i32 {
    fn from(value: AudioStreamIndex) -> Self {
        value.0
    }
}

/// The wire representation remains null/default, -1/off, or a track index.
#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize)]
#[serde(try_from = "Option<i32>", into = "Option<i32>")]
pub(super) enum SubtitleSelection {
    #[default]
    Default,
    Off,
    Track(SubtitleStreamIndex),
}
impl TryFrom<Option<i32>> for SubtitleSelection {
    type Error = &'static str;
    fn try_from(value: Option<i32>) -> Result<Self, Self::Error> {
        match value {
            None => Ok(Self::Default),
            Some(-1) => Ok(Self::Off),
            Some(index) if index >= 0 => Ok(Self::Track(SubtitleStreamIndex(index))),
            _ => Err("subtitle index must be -1 or non-negative"),
        }
    }
}
impl From<SubtitleSelection> for Option<i32> {
    fn from(value: SubtitleSelection) -> Self {
        match value {
            SubtitleSelection::Default => None,
            SubtitleSelection::Off => Some(-1),
            SubtitleSelection::Track(index) => Some(index.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positions_reject_invalid_numbers_and_preserve_fractional_seconds() {
        for seconds in [
            -1.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            922_337_203_685.477_5,
        ] {
            assert!(PlaybackPosition::try_from(seconds).is_err());
        }
        assert_eq!(PlaybackPosition::try_from(0.0).unwrap().jellyfin_ticks(), 0);
        assert_eq!(
            PlaybackPosition::try_from(1.25).unwrap().jellyfin_ticks(),
            12_500_000
        );
    }
    #[test]
    fn track_selection_preserves_the_wire_conventions() {
        for value in ["null", "-1", "0", "7"] {
            let selection: SubtitleSelection = serde_json::from_str(value).unwrap();
            assert_eq!(serde_json::to_string(&selection).unwrap(), value);
        }
        assert!(serde_json::from_str::<SubtitleSelection>("-2").is_err());
        assert!(serde_json::from_str::<AudioStreamIndex>("-1").is_err());
    }
}
