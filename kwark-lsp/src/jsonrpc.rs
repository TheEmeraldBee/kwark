use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// The ID of a request
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ID {
    Str(String),
    Num(u64),
    None,
}

/// The version of JsonRPC we're using
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum Version {
    #[serde(rename = "2.0")]
    V2,
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "2.0")
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Request {
    jsonrpc: Version,
    method: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,

    id: ID,
}

pub struct Response {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_request_version_serialize() {
        assert_eq!(
            "\"2.0\"",
            serde_json::to_string(&Version::V2).expect("should be able to serialize")
        )
    }

    #[test]
    fn test_request_version_deserialization() {
        assert_eq!(
            Version::V2,
            serde_json::from_str::<Version>("\"2.0\"").expect("should be able to deserialize")
        );
    }
}
