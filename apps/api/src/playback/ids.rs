use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore as _;
use serde::{Deserialize, Serialize};
use std::fmt;

fn random_id() -> String {
    let mut bytes = [0_u8; 24];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct SessionId(String);
impl SessionId {
    pub(super) fn generate() -> Self {
        Self(random_id())
    }
}
impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct DiscoveryId(String);
impl DiscoveryId {
    pub(super) fn generate() -> Self {
        Self(random_id())
    }
}
impl fmt::Display for DiscoveryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct CandidateId(String);
impl CandidateId {
    pub(super) fn generate() -> Self {
        Self(random_id())
    }
}
impl fmt::Display for CandidateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct ResourceId(String);
impl ResourceId {
    pub(super) fn generate() -> Self {
        Self(random_id())
    }
}
impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl SessionId {
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
    pub(super) fn media_path(&self) -> String {
        format!("/v1/playback/sessions/{self}/media")
    }
    pub(super) fn resource_path(&self, resource: impl fmt::Display) -> String {
        format!("/v1/playback/sessions/{self}/resources/{resource}")
    }
}

impl ResourceId {
    pub(super) fn from_wire(value: String) -> Self {
        Self(value)
    }
}
