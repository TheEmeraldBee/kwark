use std::fmt::Display;

use serde::{Deserialize, Serialize, de::Visitor};

/// The ID of a request
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
pub enum ID {
    Str(String),
    Num(u64),
    None,
}

/// The version of JsonRPC we're using (unit struct serializes to "2.0")
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Version;

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("2.0")
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(VersionVisitor)
    }
}

struct VersionVisitor;

impl<'de> Visitor<'de> for VersionVisitor {
    type Value = Version;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "2.0")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v == "2.0" {
            Ok(Version)
        } else {
            Err(serde::de::Error::custom("invalid version"))
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "2.0")
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Request {
    jsonrpc: Version,
    pub method: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,

    pub id: ID,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
pub struct Notification {
    jsonrpc: Version,
    pub method: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// A Response to a request
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum Response {
    Success {
        jsonrpc: Version,

        result: serde_json::Value,

        id: ID,
    },
    Error {
        jsonrpc: Version,

        error: Error,

        id: ID,
    },
}

/// An error from a request
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Error {
    pub code: i64,
    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Error {
    /// Invalid JSON was received
    pub const PARSE_ERROR: i64 = -32700;
    /// The JSON sent is not a valid request
    pub const INVALID_REQUEST: i64 = -32600;
    /// The method does not exist or is not available
    pub const METHOD_NOT_FOUND: i64 = -32601;
    /// Invalid method parameters
    pub const INVALID_PARAMS: i64 = -32602;
    /// Internal JSON-RPC error
    pub const INTERNAL_ERROR: i64 = -32603;

    /// A request was received before the server was initialized
    pub const SERVER_NOT_INITIALIZED: i64 = -32002;
    /// An unknown error
    pub const UNKNOWN_ERROR_CODE: i64 = -32001;

    /// A syntactically valid request failed
    pub const REQUEST_FAILED: i64 = -32803;
    /// The server cancelled the request
    pub const SERVER_CANCELLED: i64 = -32802;
    /// The document changed and the result is no longer valid
    pub const CONTENT_MODIFIED: i64 = -32801;
    /// The client cancelled the request
    pub const REQUEST_CANCELLED: i64 = -32800;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_version_serialize() {
        assert_eq!(
            "\"2.0\"",
            serde_json::to_string(&Version).expect("should be able to serialize")
        )
    }

    #[test]
    fn test_version_deserialization() {
        assert_eq!(
            Version,
            serde_json::from_str::<Version>("\"2.0\"").expect("should be able to deserialize")
        );
    }

    #[test]
    fn test_version_fail_deserialize() {
        assert!(serde_json::from_str::<Version>("\"hi\"").is_err());
        assert!(serde_json::from_str::<Version>("2.0").is_err());
    }

    #[test]
    fn test_request_serialize() {
        assert_eq!(
            r#"{"jsonrpc":"2.0","method":"my_method","id":null}"#,
            serde_json::to_string(&Request {
                jsonrpc: Version,
                method: "my_method".to_string(),
                params: None,
                id: ID::None,
            })
            .expect("Should be able to serialize")
        )
    }

    #[test]
    fn test_request_deserialize() {
        assert_eq!(
            Request {
                jsonrpc: Version,
                method: "my_method".to_string(),
                params: None,
                id: ID::None,
            },
            serde_json::from_str::<Request>(r#"{"jsonrpc":"2.0","method":"my_method","id":null}"#)
                .expect("Should be able to serialize")
        )
    }

    #[test]
    fn test_response_serialize() {
        assert_eq!(
            "{\"jsonrpc\":\"2.0\",\"result\":true,\"id\":null}",
            serde_json::to_string(&Response::Success {
                jsonrpc: Version,
                result: serde_json::Value::Bool(true),
                id: ID::None,
            })
            .expect("Should be able to serialize")
        );
        assert_eq!(
            "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-1,\"message\":\"No method\"},\"id\":null}",
            serde_json::to_string(&Response::Error {
                jsonrpc: Version,
                error: Error {
                    code: -1,
                    message: "No method".to_string(),
                    data: None
                },
                id: ID::None,
            })
            .expect("Should be able to serialize")
        );
    }

    #[test]
    fn test_response_deserialize() {
        assert_eq!(
            Response::Success {
                jsonrpc: Version,
                result: serde_json::Value::Bool(true),
                id: ID::None,
            },
            serde_json::from_str::<Response>("{\"jsonrpc\":\"2.0\",\"result\":true,\"id\":null}")
                .expect("Should be able to serialize")
        );
        assert_eq!(
            Response::Error {
                jsonrpc: Version,
                error: Error {
                    code: -1,
                    message: "No method".to_string(),
                    data: None
                },
                id: ID::None,
            },
            serde_json::from_str::<Response>("{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-1,\"message\":\"No method\"},\"id\":null}")
            .expect("Should be able to serialize")
        );

        // Fails because error & result exist
        assert!(
            serde_json::from_str::<Response>(
                "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-1,\"message\":\"No method\"},\"result\":true,\"id\":null}"
            ).is_err()
        );

        // Fails because neither error nor result exist
        assert!(serde_json::from_str::<Response>("{\"jsonrpc\":\"2.0\",\"id\":1}").is_err());

        // A null result is still a success
        assert_eq!(
            Response::Success {
                jsonrpc: Version,
                result: serde_json::Value::Null,
                id: ID::Num(1),
            },
            serde_json::from_str::<Response>("{\"jsonrpc\":\"2.0\",\"result\":null,\"id\":1}")
                .expect("Should be able to deserialize")
        );
    }
}
