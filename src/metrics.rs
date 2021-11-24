use lazy_static::lazy_static;
use prometheus::{
    register_int_counter, register_int_gauge, register_int_gauge_vec, IntCounter, IntGauge,
    IntGaugeVec,
};

pub struct Metrics {
    pub subscriptions_count: IntGaugeVec,
    pub connections_count: IntGauge,
    pub slot: IntGauge,
    pub account_updates_count: IntCounter,
    pub slot_updates_count: IntCounter,
    pub bytes_received: IntCounter,
    pub bytes_sent: IntCounter,
    pub connection_timeouts: IntCounter,
}

lazy_static! {
    pub static ref METRICS: Metrics = {
        let subscriptions_count = register_int_gauge_vec!(
            "subscriptions_count",
            "Number of subscriptions tracked by each subscription manager",
            &["manager_id"]
        )
        .unwrap();

        let connections_count = register_int_gauge!(
            "connections_count",
            "Number of currently active web socket connections from clients"
        )
        .unwrap();

        let slot = register_int_gauge!("slot", "Max slot number, received from pubsub").unwrap();

        let account_updates_count = register_int_counter!(
            "account_updates_count",
            "Total number of account updates received from pubsub"
        )
        .unwrap();

        let slot_updates_count = register_int_counter!(
            "slot_updates_count",
            "Total number of slot updates received from pubsub"
        )
        .unwrap();

        let bytes_received = register_int_counter!(
            "bytes_received",
            "Total number of bytes received from pubsub"
        )
        .unwrap();

        let bytes_sent = register_int_counter!(
            "bytes_sent",
            "Total number of notification bytes sent to clients"
        )
        .unwrap();

        let connection_timeouts = register_int_counter!(
            "connection_timeouts",
            "Total number of websocket connections which were terminated due to client inactivity"
        )
        .unwrap();

        Metrics {
            subscriptions_count,
            connections_count,
            slot,
            account_updates_count,
            slot_updates_count,
            bytes_received,
            bytes_sent,
            connection_timeouts,
        }
    };
}
