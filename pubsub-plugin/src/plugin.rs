use std::{convert::TryFrom, fmt::Debug, fs::File, sync::Arc};

use serde::Deserialize;
use solana_accountsdb_plugin_interface::accountsdb_plugin_interface::{
    AccountsDbPlugin, AccountsDbPluginError, ReplicaAccountInfoVersions, Result as AccDBResult,
    SlotStatus as AccDBSlotStatus,
};
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc::Sender,
};
use tokio_nsq::{NSQEvent, NSQProducerConfig, NSQTopic};

use crate::{publisher::Publisher, Message};

impl Default for Topics {
    fn default() -> Self {
        let accounts = NSQTopic::new("mopics").unwrap();
        let slots = NSQTopic::new("topics").unwrap();

        Self { accounts, slots }
    }
}

impl TryFrom<&str> for NSQPluginConfig {
    type Error = AccountsDbPluginError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let file = File::open(value).map_err(AccountsDbPluginError::ConfigFileOpenError)?;
        let config = serde_json::from_reader(file)
            .map_err(|e| AccountsDbPluginError::ConfigFileReadError { msg: e.to_string() })?;
        Ok(config)
    }
}

impl Debug for NSQPubSubPlugin {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
/// Configuration that the plugin was loaded with
#[derive(Deserialize, Default, Debug)]
struct NSQPluginConfig {
    account_topic: String,
    slot_topic: String,
    host: String,
}

struct Topics {
    accounts: Arc<NSQTopic>,
    slots: Arc<NSQTopic>,
}

pub struct NSQPubSubPlugin {
    runtime: Runtime,
    publishers: Vec<Sender<Message>>,
    host: String,
    topics: Topics,
}

impl NSQPubSubPlugin {
    fn new() -> Self {
        let runtime = Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("nsq-pubsub-plugin")
            .enable_all()
            .build();

        if let Ok(runtime) = runtime {
            Self {
                runtime,
                publishers: Vec::new(),
                host: String::new(),
                topics: Topics::default(), // placholder
            }
        } else {
            std::process::exit(1);
        }
    }

    fn get_publisher(&mut self) -> &Sender<Message> {
        let mut index = 0;
        for p in &self.publishers {
            if p.is_closed() {
                // the only reason why it might have shut down,
                // is that connection to nsq server has been terminated
                std::process::exit(1);
            }
            if p.capacity() > 0 {
                break;
            }
            index += 1;
        }
        if index < self.publishers.len() {
            return &self.publishers[index];
        }
        // if no free workers are available we have to
        // create another one to handle the load
        let mut producer = NSQProducerConfig::new(&self.host).build();
        if let Some(NSQEvent::Unhealthy()) = self.runtime.block_on(producer.consume()) {
            std::process::exit(1);
        }
        let publisher = Publisher::new(producer);
        self.publishers.push(publisher.0);
        self.runtime.spawn(publisher.1.run());
        self.publishers.last().unwrap()
    }

    fn publish(&mut self, msg: Message) {
        let publisher = self.get_publisher();
        if let Err(_) = publisher.try_send(msg) {
            std::process::exit(1);
        }
    }
}

impl AccountsDbPlugin for NSQPubSubPlugin {
    fn name(&self) -> &'static str {
        "NSQPubSubPlugin"
    }

    fn on_load(&mut self, config_file: &str) -> AccDBResult<()> {
        let config = NSQPluginConfig::try_from(config_file)?;
        let accounts = NSQTopic::new(&config.account_topic).unwrap();
        let slots = NSQTopic::new(&config.slot_topic).unwrap();
        self.topics = Topics { accounts, slots };

        Ok(())
    }

    fn update_account(
        &mut self,
        account: ReplicaAccountInfoVersions<'_>,
        slot: u64,
        is_startup: bool,
    ) -> AccDBResult<()> {
        // for now we are not interested in accounts, which were restored from last snapshot
        if is_startup {
            return Ok(());
        }
        let message = Message::from_account(account, slot, Arc::clone(&self.topics.accounts));
        self.publish(message);
        Ok(())
    }

    fn notify_end_of_startup(&mut self) -> AccDBResult<()> {
        Ok(())
    }

    fn update_slot_status(
        &mut self,
        slot: u64,
        parent: Option<u64>,
        status: AccDBSlotStatus,
    ) -> AccDBResult<()> {
        let message = Message::from_slot(
            slot,
            parent.unwrap_or_default(),
            status,
            Arc::clone(&self.topics.slots),
        );
        self.publish(message);
        Ok(())
    }
}

/// Every AccountsDbPlugin is required to expose this method, in order for the validator
/// to be able to load them during runtime
#[no_mangle]
#[allow(improper_ctypes_definitions)]
/// # Safety
///
/// This function returns the AccountsDbPluginClickHouse pointer as trait AccountsDbPlugin.
pub unsafe extern "C" fn _create_plugin() -> *mut dyn AccountsDbPlugin {
    let plugin = NSQPubSubPlugin::new();
    let plugin: Box<dyn AccountsDbPlugin> = Box::new(plugin);
    let b = Box::into_raw(plugin);
    b
}
