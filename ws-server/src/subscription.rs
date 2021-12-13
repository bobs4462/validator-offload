use crate::{error::SubError, Commitment, Pubkey, SubID, JSONRPC};
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize,
};
use serde_json::Value as JsonValue;

/// Represent all kinds of supported requests that the client
/// may send over websocket connection
#[derive(Deserialize)]
pub struct SubRequest {
    /// Identifier of request, used when sending response back
    pub id: u64,
    /// Request method that should be performed on server,
    /// e.g. subscribe or unsubscribe for variouse updates
    pub method: Method,
    /// Parameters of request, that are required by method
    #[serde(default)]
    pub params: Params,
}

/// List of supported methods
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Method {
    /// Subscribe for account
    AccountSubscribe,
    /// Subscribe for accounts of given program
    ProgramSubscribe,
    /// Unsubscribe from account
    AccountUnsubscribe,
    /// Unsubscribe from accounts of a given program
    ProgramUnsubscribe,
    /// Subscribe for slot updates
    SlotSubscribe,
    /// Unsubscribe from slot updates
    SlotUnsubscribe,
}

/// Various formats of request parameters, that different methods require
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub enum Params {
    /// Parameter for all subscriptions methods.
    /// _Note:_ slot subscriptions do not require any parameters
    SubscribeParams(PubkeyParams),
    /// Parameter for all unsubscription methods.
    /// Only client issued id is required
    UnsubscribeParams(SubID),
    /// Parameters weren't supplied
    Absent,
}

/// Request parameters for account or program subscription
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct PubkeyParams {
    /// Public key of entity, for which client wants to receive updates
    pub pubkey: Pubkey,
    /// Extra subscription options
    pub options: SubOptions,
}

/// Options for configuring subscription
#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct SubOptions {
    /// Encoding to encode account data in, before
    /// sending notification to client
    pub encoding: Encoding,
    /// Commitment level, which must be reached by account's
    /// slot, before sending notification to client
    #[serde(default)]
    pub commitment: Commitment,
}

/// Various encoding options, that the client might
/// want to receive the notification in
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub enum Encoding {
    /// base58 encoding
    Base58,
    /// base64 encoding
    Base64,
    /// base64 encoding, with additional zstd compression
    #[serde(rename = "base64+zstd")]
    Base64Zstd,
}

/// Response that must be sent to client over websocket
/// connection, after receiving any request
#[derive(Serialize)]
pub struct SubResponse {
    jsonrpc: &'static str,
    id: u64,
    result: SubResult,
}

/// Error response that is sent to client, in case if
/// the request couldn't be handled by server
#[derive(Serialize)]
pub struct SubResponseError<'a> {
    jsonrpc: &'static str,
    id: Option<u64>,
    error: SubError<'a>,
}

/// Result of processing client request
#[derive(Serialize)]
#[serde(untagged)]
pub enum SubResult {
    /// Id of subscription, which was created on server
    Id(SubID),
    /// Indicates success status of unsubscription request
    Status(bool),
}

impl Params {
    /// try to get subscription parameters for account or program,
    /// if the current request was a to create a new subscription
    pub fn sub(self) -> Option<PubkeyParams> {
        if let Self::SubscribeParams(params) = self {
            return Some(params);
        }
        None
    }
    /// Try to get id of subscription to remove,if the current
    /// request was a to remove existing subscription
    pub fn unsub(self) -> Option<SubID> {
        if let Self::UnsubscribeParams(id) = self {
            return Some(id);
        }
        None
    }
}

impl Default for Params {
    fn default() -> Self {
        Self::Absent
    }
}

impl SubResponse {
    /// Construct a new response
    pub fn new(id: u64, result: SubResult) -> Self {
        Self {
            jsonrpc: JSONRPC,
            id,
            result,
        }
    }
}

impl<'a> SubResponseError<'a> {
    /// Construct a new response indicating error
    pub fn new(id: Option<u64>, error: SubError<'a>) -> Self {
        Self {
            jsonrpc: JSONRPC,
            id,
            error,
        }
    }
}

impl<'de> Deserialize<'de> for Params {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ParamsVisitor;

        use std::fmt::{self, Formatter};
        impl<'de> Visitor<'de> for ParamsVisitor {
            type Value = Params;

            fn expecting(&self, f: &mut Formatter<'_>) -> fmt::Result {
                f.write_str("Array of [<pubkey | string>, <options | object>]")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let first: JsonValue = seq
                    .next_element()
                    .ok()
                    .flatten()
                    .ok_or_else(|| de::Error::missing_field("<pubkey | string>"))?;
                if first.is_u64() {
                    let sub = first.as_u64().unwrap();
                    return Ok(Params::UnsubscribeParams(sub));
                }

                let pubkey = first
                    .as_str()
                    .ok_or_else(|| de::Error::missing_field("<pubkey | string>"))?;

                let mut buf = [0; 32];

                bs58::decode(pubkey)
                    .into(&mut buf)
                    .map_err(|_| de::Error::custom("wrong public key"))?;
                let pubkey: Pubkey = buf;

                let options: JsonValue = seq
                    .next_element()
                    .ok()
                    .flatten()
                    .ok_or_else(|| de::Error::missing_field("<options | object >"))?;
                let options = SubOptions::deserialize(options)
                    .map_err(|_| de::Error::custom("incorrect format of parameter options"))?;
                let params = PubkeyParams { pubkey, options };
                Ok(Params::SubscribeParams(params))
            }
        }
        deserializer.deserialize_seq(ParamsVisitor)
    }
}
