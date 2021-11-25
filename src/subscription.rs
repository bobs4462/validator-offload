use crate::{error::SubError, Commitment, Pubkey, SubID, JSONRPC};
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize,
};
use serde_json::Value as JsonValue;

#[derive(Deserialize)]
pub struct SubRequest {
    pub id: u64,
    pub method: Method,
    #[serde(default)]
    pub params: Params,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Method {
    AccountSubscribe,
    ProgramSubscribe,
    AccountUnsubscribe,
    ProgramUnsubscribe,
    SlotSubscribe,
    SlotUnsubscribe,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub enum Params {
    SubscribeParams(PubkeyParams),
    UnsubscribeParams(SubID),
    Absent,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct PubkeyParams {
    pub pubkey: Pubkey,
    pub options: SubOptions,
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct SubOptions {
    encoding: Encoding,
    #[serde(default)]
    pub commitment: Commitment,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
enum Encoding {
    Base58,
    Base64,
    #[serde(rename = "base64+zstd")]
    Base64Zstd,
}

#[derive(Serialize)]
pub struct SubResponse {
    jsonrpc: &'static str,
    id: u64,
    result: SubResult,
}

#[derive(Serialize)]
pub struct SubResponseError<'a> {
    jsonrpc: &'static str,
    id: Option<u64>,
    error: SubError<'a>,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum SubResult {
    Id(SubID),
    Status(bool),
}

impl Params {
    pub fn sub(self) -> Option<PubkeyParams> {
        if let Self::SubscribeParams(params) = self {
            return Some(params);
        }
        None
    }
    pub fn unsub(self) -> Option<SubID> {
        if let Self::UnsubscribeParams(id) = self {
            return Some(id);
        }
        None
    }
}

impl Default for Params {
    fn default() -> Self {
        return Self::Absent;
    }
}

impl SubResponse {
    pub fn new(id: u64, result: SubResult) -> Self {
        Self {
            jsonrpc: JSONRPC,
            id,
            result,
        }
    }
}

impl<'a> SubResponseError<'a> {
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

            fn expecting(&self, f: &mut Formatter) -> fmt::Result {
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
                    .ok_or(de::Error::missing_field("<pubkey | string>"))?;
                if first.is_u64() {
                    let sub = first.as_u64().unwrap();
                    return Ok(Params::UnsubscribeParams(sub));
                }

                let pubkey = first
                    .as_str()
                    .ok_or(de::Error::missing_field("<pubkey | string>"))?;

                let mut buf = [0; 32];

                bs58::decode(pubkey)
                    .into(&mut buf)
                    .map_err(|_| de::Error::custom("wrong public key"))?;
                let pubkey: Pubkey = buf;

                let options: JsonValue = seq
                    .next_element()
                    .ok()
                    .flatten()
                    .ok_or(de::Error::missing_field("<options | object >"))?;
                let options = SubOptions::deserialize(options)
                    .map_err(|_| de::Error::custom("incorrect format of parameter options"))?;
                let params = PubkeyParams { pubkey, options };
                Ok(Params::SubscribeParams(params))
            }
        }
        deserializer.deserialize_seq(ParamsVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_account_subscribe() {
        let request = r#"
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "accountSubscribe",
            "params": [
                "CM78CPUeXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNH12",
                {
                    "encoding": "base64",
                    "commitment": "processed"
                }
            ]
        }
        "#;
        let parsed: SubRequest = serde_json::from_str(request).unwrap();
        assert_eq!(parsed.method, Method::AccountSubscribe);
        let mut pubkey = [0; 32];
        bs58::decode("CM78CPUeXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNH12")
            .into(&mut pubkey)
            .unwrap();
        assert_eq!(
            parsed.params,
            Params::SubscribeParams(PubkeyParams {
                pubkey,
                options: SubOptions {
                    encoding: Encoding::Base64,
                    commitment: Commitment::Processed
                }
            })
        );
    }
    #[test]
    fn parse_programs_subscribe_without_commitment() {
        let request = r#"
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "programSubscribe",
            "params": [
                "CM78CPUeXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNH12",
                {
                    "encoding": "base64+zstd"
                }
            ]
        }
        "#;
        let parsed: SubRequest = serde_json::from_str(request).unwrap();
        assert_eq!(parsed.method, Method::ProgramSubscribe);
        let mut pubkey = [0; 32];
        bs58::decode("CM78CPUeXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNH12")
            .into(&mut pubkey)
            .unwrap();
        assert_eq!(
            parsed.params,
            Params::SubscribeParams(PubkeyParams {
                pubkey,
                options: SubOptions {
                    encoding: Encoding::Base64Zstd,
                    commitment: Commitment::Finalized
                }
            })
        );
    }
    #[test]
    fn parse_slot_subscribe() {
        let request = r#"{"jsonrpc":"2.0", "id":1, "method":"slotSubscribe"}"#;
        let parsed: SubRequest = serde_json::from_str(request).unwrap();
        assert!(parsed.params.sub().is_none());
        assert_eq!(parsed.method, Method::SlotSubscribe);
    }
    #[test]
    fn parse_unsubscribe() {
        let request = r#"{"jsonrpc":"2.0", "id":1, "method":"accountUnsubscribe", "params":[0]}"#;
        let parsed: SubRequest = serde_json::from_str(request).unwrap();
        assert_eq!(parsed.method, Method::AccountUnsubscribe);
        assert_eq!(parsed.params, Params::UnsubscribeParams(0));
    }
}
