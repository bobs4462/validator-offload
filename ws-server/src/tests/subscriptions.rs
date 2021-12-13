#![cfg(test)]
use std::hash::Hash;

use crate::{
    buffer::Buffer,
    manager::{SubscriptionManager, SubscriptionsRouter},
    message::{AccountUpdatedMessage, SlotUpdatedMessage, SubscribeMessage, SubscriptionInfo},
    subscription::*,
    Commitment, SubKey, SubscriptionKind,
};
use actix::{Actor, Addr, Context, Handler, Message};

#[derive(Message)]
#[rtype(result = "usize")]
enum CountRequestMessage {
    AccountSubscriptionsCount(SubKey),
    SlotSubscriptionsCount,
}

#[derive(Message)]
#[rtype(result = "Addr<SubscriptionManager>")]
struct GetAddr<T: Hash>(T);

struct DummyActor;
impl Actor for DummyActor {
    type Context = Context<Self>;
}

impl Handler<AccountUpdatedMessage> for DummyActor {
    type Result = ();
    fn handle(&mut self, _: AccountUpdatedMessage, _: &mut Self::Context) -> Self::Result {}
}

impl Handler<SlotUpdatedMessage> for DummyActor {
    type Result = ();
    fn handle(&mut self, _: SlotUpdatedMessage, _: &mut Self::Context) -> Self::Result {}
}

impl Handler<CountRequestMessage> for SubscriptionManager {
    type Result = usize;

    fn handle(&mut self, msg: CountRequestMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            CountRequestMessage::SlotSubscriptionsCount => self.slot_sub_count(),
            CountRequestMessage::AccountSubscriptionsCount(key) => self.account_sub_count(&key),
        }
    }
}

impl<T: Hash> Handler<GetAddr<T>> for SubscriptionsRouter {
    type Result = Addr<SubscriptionManager>;

    fn handle(&mut self, msg: GetAddr<T>, _ctx: &mut Self::Context) -> Self::Result {
        self.addr(msg.0).clone()
    }
}

#[actix::test]
async fn test_routing() {
    let router = SubscriptionsRouter::new(4);
    let subkey = SubKey {
        key: [1; 32],
        commitment: Commitment::Processed,
        kind: SubscriptionKind::Account,
    };
    let handler = DummyActor.start();

    router.do_send(SubscribeMessage::AccountSubscribe(SubscriptionInfo {
        key: subkey.clone(),
        recipient: handler.clone().recipient(),
    }));
    let addr = router.send(GetAddr(subkey.clone())).await.unwrap();
    let mut acc_sub_count = addr
        .send(CountRequestMessage::AccountSubscriptionsCount(
            subkey.clone(),
        ))
        .await
        .unwrap();
    assert_eq!(acc_sub_count, 1);
    router
        .send(SubscribeMessage::AccountUnsubscribe(SubscriptionInfo {
            key: subkey.clone(),
            recipient: handler.clone().recipient(),
        }))
        .await
        .unwrap();
    acc_sub_count = addr
        .send(CountRequestMessage::AccountSubscriptionsCount(
            subkey.clone(),
        ))
        .await
        .unwrap();
    assert_eq!(acc_sub_count, 0);

    router.do_send(SubscribeMessage::SlotSubscribe(handler.clone().recipient()));
    let addr = router
        .send(GetAddr(handler.clone().recipient::<SlotUpdatedMessage>()))
        .await
        .unwrap();
    let mut slot_sub_count = addr
        .send(CountRequestMessage::SlotSubscriptionsCount)
        .await
        .unwrap();
    assert_eq!(slot_sub_count, 1);
    router
        .send(SubscribeMessage::SlotUnsubscribe(
            handler.clone().recipient(),
        ))
        .await
        .unwrap();
    slot_sub_count = addr
        .send(CountRequestMessage::SlotSubscriptionsCount)
        .await
        .unwrap();
    assert_eq!(slot_sub_count, 0);
}
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
