mod library;
mod reader;
mod utils;

#[macro_use]
extern crate prettytable;
use env_logger;
use library::{last_five, parse_put};
use log::info;
use reader::read_feeds;
use serde_json::Value;
use slack::{Event, EventHandler, Message, RtmClient};
struct Handler;

#[derive(Clone, Debug)]
pub enum SlackChannel {
    Aws,
    Rust,
    Kubernetes,
    Python,
    BattleBots,
    Library,
}

impl SlackChannel {
    pub fn id(&self) -> &'static str {
        match self {
            SlackChannel::Aws => "CA6MUA4LU",
            SlackChannel::Rust => "C8EHWNKHV",
            SlackChannel::Kubernetes => "C91DM9Y6S",
            SlackChannel::Python => "C6DTBQK4P",
            SlackChannel::BattleBots => "CD31RPEFR",
            SlackChannel::Library => "CE2L5QUGP",
        }
    }
}

#[allow(unused_variables)]
impl EventHandler for Handler {
    fn on_event(&mut self, client: &RtmClient, event: Event) {
        info!("on_event(event: {:?})", event);

        if let Event::Message(message) = event.clone() {
            self.handle_message(*message, client, &event)
        }
    }

    fn on_close(&mut self, client: &RtmClient) {}

    fn on_connect(&mut self, client: &RtmClient) {
        std::thread::spawn(read_feeds);
    }
}

#[allow(unused_variables)]
impl Handler {
    fn handle_message(&mut self, message: Message, client: &RtmClient, event: &Event) {
        let message_standard = match message {
            Message::Standard(message_standard) => message_standard,
            _ => return,
        };

        let channel: String = message_standard.channel.unwrap();
        let user: String = message_standard.user.unwrap();
        let bot_id: &str = client
            .start_response()
            .slf
            .as_ref()
            .unwrap()
            .id
            .as_ref()
            .unwrap();

        let text: String = message_standard.text.unwrap();
        if channel == SlackChannel::Library.id() {
            info!("recognized message from #library");

            if text.starts_with("!put ") {
                info!("matched !put");
                parse_put(&text, &user)
            } else if text.starts_with("!last") {
                info!("matched !last");
                last_five()
            }
        }

        if text.contains(bot_id) {
            info!("is a mention");
            respond_hi(&bot_id, &text, &channel, &client);
        }
    }
}

fn respond_hi(bot_id: &str, text: &str, channel: &str, client: &RtmClient) {
    let pattern = format!("<@{}> hi", bot_id);

    if text.contains(&pattern) {
        let _ = client.sender().send_message(channel, "Hi there!");
    }
}

fn main() {
    env_logger::init();
    // https://github.com/emk/rust-musl-builder#making-openssl-work
    openssl_probe::init_ssl_cert_env_vars();

    // get bot token from environment variables
    let target_env_var = "SLACKBOT_TOKEN_SECRET";
    let mut api_key_json: String = String::new();
    for (k, v) in std::env::vars() {
        if k == target_env_var {
            api_key_json = v;
        }
    }

    if api_key_json.is_empty() {
        eprintln!(
            "no {} environment variable found!\nPlease set this env var and try again.",
            target_env_var
        );
        std::process::exit(1);
    }

    let slackbot_token_json: Value = serde_json::from_str(&api_key_json).unwrap();
    let slackbot_token = slackbot_token_json["SLACKBOT_TOKEN"].as_str();

    let mut handler = Handler;
    let r = RtmClient::login_and_run(slackbot_token.unwrap(), &mut handler);
    match r {
        Ok(_) => {}
        Err(err) => panic!("Error: {}", err),
    }
}
