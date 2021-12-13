use std::time::Duration;

use tokio::sync::mpsc::{Receiver, Sender};
use tokio_nsq::{NSQEvent, NSQProducer};

use crate::Message;

pub(super) struct Publisher {
    producer: NSQProducer,
    receiver: Receiver<Message>,
}

pub(super) enum Error {
    SerializeError,
    PublishTimeout,
}

impl Publisher {
    pub(super) fn new(producer: NSQProducer) -> (Sender<Message>, Self) {
        let (sender, receiver) = tokio::sync::mpsc::channel(4096);
        let publisher = Self { producer, receiver };
        (sender, publisher)
    }

    pub(super) async fn run(mut self) -> Result<(), Error> {
        while let Some(msg) = self.receiver.recv().await {
            self.send(msg).await?;
        }
        Ok(())
    }

    async fn send(&mut self, msg: Message) -> Result<(), Error> {
        let mut value = match msg.serialize() {
            Some(v) => v,
            None => return Err(Error::SerializeError),
        };
        let mut retries = 0;
        loop {
            while let Err(e) = self.producer.publish(&msg.topic, value) {
                eprintln!("couldn't publish to nsq: {}", e);
                retries += 1;
                if retries >= 10 {
                    eprintln!("retries exceeded to publish nsq, shutting down worker");
                    return Err(Error::PublishTimeout);
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                value = msg.serialize().unwrap();
            }
            if let Some(NSQEvent::Ok()) = self.producer.consume().await {
                break;
            }
            value = msg.serialize().unwrap();
        }
        Ok(())
    }
}
