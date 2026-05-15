use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::Receiver;

use crate::{
    channels::{Notify, WebhookChannel},
    config::DSLRule,
};
use serde_json::Value;

pub struct Notifier<'a> {
    webhook_channels: HashMap<String, WebhookChannel<'a>>,
}

impl<'a> Notifier<'a> {
    pub fn new(webhook_channels: HashMap<String, WebhookChannel<'a>>) -> Self {
        Self { webhook_channels }
    }

    pub async fn run(&self, mut rx: Receiver<(Arc<DSLRule>, Value)>) {
        loop {
            let notification = match rx.recv().await {
                Some(n) => n,
                None => {
                    continue;
                }
            };

            for (channel_name, _custom_message) in notification.0.channels.iter() {
                if let Some(channel) = self.webhook_channels.get(channel_name) {
                    let generated_message =
                        match channel.message_template.render("t", &notification.1) {
                            Ok(n) => n,
                            Err(e) => {
                                eprintln!("Error rendering message: {:?}", e);
                                continue;
                            }
                        };

                    if let Err(e) = channel.notify(generated_message).await {
                        eprintln!("Failed to send notification: {:?}", e);
                    }
                }
            }
        }
    }
}
