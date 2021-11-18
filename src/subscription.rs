use crate::{Commitment, Pubkey, SubID};
use serde::{
    de::{self, Visitor},
    Deserialize,
};
use serde_json::Value as JsonValue;

#[derive(Deserialize)]
pub struct SubRequest {
    id: u64,
    method: Method,
    params: Option<SubParams>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub enum Method {
    AccountSubscribe,
    ProgramSubscribe,
    AccountUnsubscribe,
    ProgramUnsubscribe,
    SlotSubscribe,
    SlotUnsubscribe,
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub enum SubParams {
    AccountOrProgramParams(PubkeyParams),
    UnsubscribeParams(SubID),
}

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct PubkeyParams {
    pubkey: Pubkey,
    options: SubOptions,
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
struct SubOptions {
    encoding: Encoding,
    #[serde(default)]
    commitment: Commitment,
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

impl<'de> Deserialize<'de> for SubParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ParamsVisitor;

        use std::fmt::{self, Formatter};
        impl<'de> Visitor<'de> for ParamsVisitor {
            type Value = SubParams;

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
                    return Ok(SubParams::UnsubscribeParams(sub));
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
                Ok(SubParams::AccountOrProgramParams(params))
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
        assert!(parsed.params.is_some());
        assert_eq!(parsed.method, Method::AccountSubscribe);
        let params = parsed.params.unwrap();
        let mut pubkey = [0; 32];
        bs58::decode("CM78CPUeXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNH12")
            .into(&mut pubkey)
            .unwrap();
        assert_eq!(
            params,
            SubParams::AccountOrProgramParams(PubkeyParams {
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
        assert!(parsed.params.is_some());
        assert_eq!(parsed.method, Method::ProgramSubscribe);
        let params = parsed.params.unwrap();
        let mut pubkey = [0; 32];
        bs58::decode("CM78CPUeXjn8o3yroDHxUtKsZZgoy4GPkPPXfouKNH12")
            .into(&mut pubkey)
            .unwrap();
        assert_eq!(
            params,
            SubParams::AccountOrProgramParams(PubkeyParams {
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
        assert!(parsed.params.is_none());
        assert_eq!(parsed.method, Method::SlotSubscribe);
    }
    #[test]
    fn parse_unsubscribe() {
        let request = r#"{"jsonrpc":"2.0", "id":1, "method":"accountUnsubscribe", "params":[0]}"#;
        let parsed: SubRequest = serde_json::from_str(request).unwrap();
        assert!(parsed.params.is_some());
        assert_eq!(parsed.method, Method::AccountUnsubscribe);
        let params = parsed.params.unwrap();
        assert_eq!(params, SubParams::UnsubscribeParams(0));
    }
}
