#[allow(non_snake_case)]
use std::{fmt::Display, str::FromStr};

use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};


// You can use a custom type with the hash segment as long as it implements Display, FromStr and Default
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct MapState {
    /// if false, overwrite with the default value with "true" set.
    pub is_init: bool,
    pub zoom: f64,
    pub pos: (f64, f64),
}


pub const INIT_STATE: MapState = MapState {
    is_init: true,
    zoom: 14.1,
    pos: (150063.61456866315, 94921.7548560014),
};

// Display the state in a way that can be parsed by FromStr
impl Display for MapState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut serialized = Vec::new();
        if ciborium::into_writer(self, &mut serialized).is_ok() {
            write!(f, "{}", URL_SAFE.encode(serialized))?;
        }
        Ok(())
    }
}

pub enum StateParseError {
    DecodeError(base64::DecodeError),
    CiboriumError(ciborium::de::Error<std::io::Error>),
}

impl std::fmt::Display for StateParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DecodeError(err) => write!(f, "Failed to decode base64: {}", err),
            Self::CiboriumError(err) => write!(f, "Failed to deserialize: {}", err),
        }
    }
}

// Parse the state from a string that was created by Display
impl FromStr for MapState {
    type Err = StateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let decompressed = URL_SAFE
            .decode(s.as_bytes())
            .map_err(StateParseError::DecodeError)?;
        let parsed = ciborium::from_reader(std::io::Cursor::new(decompressed))
            .map_err(StateParseError::CiboriumError)?;
        Ok(parsed)
    }
}