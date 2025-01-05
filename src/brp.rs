//! Types are copied from `bevy_remote` rather than using it as a dependency due to how many crates
//! it includes that we don't need.
#![expect(dead_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The method path for a `bevy/get` request.
pub const BRP_GET_METHOD: &str = "bevy/get";

/// The method path for a `bevy/query` request.
pub const BRP_QUERY_METHOD: &str = "bevy/query";

/// The method path for a `bevy/spawn` request.
pub const BRP_SPAWN_METHOD: &str = "bevy/spawn";

/// The method path for a `bevy/insert` request.
pub const BRP_INSERT_METHOD: &str = "bevy/insert";

/// The method path for a `bevy/remove` request.
pub const BRP_REMOVE_METHOD: &str = "bevy/remove";

/// The method path for a `bevy/destroy` request.
pub const BRP_DESTROY_METHOD: &str = "bevy/destroy";

/// The method path for a `bevy/reparent` request.
pub const BRP_REPARENT_METHOD: &str = "bevy/reparent";

/// The method path for a `bevy/list` request.
pub const BRP_LIST_METHOD: &str = "bevy/list";

/// The method path for a `bevy/get+watch` request.
pub const BRP_GET_AND_WATCH_METHOD: &str = "bevy/get+watch";

/// The method path for a `bevy/list+watch` request.
pub const BRP_LIST_AND_WATCH_METHOD: &str = "bevy/list+watch";

/// A copy of `bevy_remote::BrpRequest`.
#[derive(Debug, Serialize, Clone)]
pub struct BrpRequest {
    /// This field is mandatory and must be set to `"2.0"` for the request to be accepted.
    pub jsonrpc: String,

    /// The action to be performed.
    pub method: String,

    /// Arbitrary data that will be returned verbatim to the client as part of
    /// the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,

    /// The parameters, specific to each method.
    ///
    /// These are passed as the first argument to the method handler.
    /// Sometimes params can be omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A copy of `bevy_remote::BrpResponse`.
#[derive(Debug, Deserialize, Clone)]
pub struct BrpResponse {
    /// This field is mandatory and must be set to `"2.0"`.
    pub jsonrpc: String,

    /// The id of the original request.
    pub id: Option<Value>,

    /// The actual response payload.
    #[serde(flatten)]
    pub payload: BrpPayload,
}

/// A copy of `bevy_remote::BrpPayload`.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BrpPayload {
    /// `Ok` variant
    Result(Value),

    /// `Err` variant
    Error(BrpError),
}

/// A copy of `bevy_remote::BrpError`.
#[derive(Debug, Deserialize, Clone)]
pub struct BrpError {
    /// Defines the general type of the error.
    pub code: i16,

    /// Short, human-readable description of the error.
    pub message: String,

    /// Optional additional error data.
    pub data: Option<Value>,
}

/// Modified copy of `bevy_remote::BrpQueryRow`.
#[derive(Debug, Deserialize, Clone)]
pub struct BrpQueryRow {
    /// The ID of the entity that matched.
    pub entity: u64,

    /// The serialized values of the requested components.
    pub components: HashMap<String, Value>,

    /// The boolean-only containment query results.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub has: HashMap<String, Value>,
}

/// Copt of `Lenient` varient of `bevy_remote::BrpGetResponse`.
#[derive(Debug, Deserialize, Clone)]
pub struct BrpGetResponse {
    /// A map of successful components with their values.
    pub components: HashMap<String, Value>,
    /// A map of unsuccessful components with their errors.
    pub errors: HashMap<String, Value>,
}

/// Copy of `bevy_remote::error_codes`.
pub mod error_codes {
    // JSON-RPC errors
    // Note that the range -32728 to -32000 (inclusive) is reserved by the JSON-RPC specification.

    /// Invalid JSON.
    pub const PARSE_ERROR: i16 = -32700;

    /// JSON sent is not a valid request object.
    pub const INVALID_REQUEST: i16 = -32600;

    /// The method does not exist / is not available.
    pub const METHOD_NOT_FOUND: i16 = -32601;

    /// Invalid method parameter(s).
    pub const INVALID_PARAMS: i16 = -32602;

    /// Internal error.
    pub const INTERNAL_ERROR: i16 = -32603;

    // Bevy errors (i.e. application errors)

    /// Entity not found.
    pub const ENTITY_NOT_FOUND: i16 = -23401;

    /// Could not reflect or find component.
    pub const COMPONENT_ERROR: i16 = -23402;

    /// Could not find component in entity.
    pub const COMPONENT_NOT_PRESENT: i16 = -23403;

    /// Cannot reparent an entity to itself.
    pub const SELF_REPARENT: i16 = -23404;
}
