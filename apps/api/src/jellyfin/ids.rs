//! Distinct provider identifiers. Their contents are opaque to Reel.
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JellyfinItemId(String);
impl JellyfinItemId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for JellyfinItemId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for JellyfinItemId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl fmt::Display for JellyfinItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JellyfinUserId(String);
impl JellyfinUserId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for JellyfinUserId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for JellyfinUserId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl fmt::Display for JellyfinUserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JellyfinMediaSourceId(String);
impl JellyfinMediaSourceId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for JellyfinMediaSourceId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl From<&str> for JellyfinMediaSourceId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
impl fmt::Display for JellyfinMediaSourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
