mod library;
mod reader;
mod utils;

#[macro_use]
extern crate prettytable;
use library::{last_five, parse_put};
use log::info;
use reader::read_feeds;
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
        let token = utils::get_slack_token_from_env_var();
        std::thread::spawn(|| read_feeds(token));
    }
}

#[allow(unused_variables)]
impl Handler {
    fn handle_message(&mut self, message: Message, client: &RtmClient, event: &Event) {
        let message_standard = match message {
            Message::Standard(message_standard) => message_standard,
            _ => return,
        };

        let channel: String = message_standard.channel.clone().unwrap();
        let user: String = message_standard.user.clone().unwrap();
        let bot_id: &str = client
            .start_response()
            .slf
            .as_ref()
            .unwrap()
            .id
            .as_ref()
            .unwrap();

        let text: String = message_standard.text.clone().unwrap();

        if channel == SlackChannel::Library.id() || channel == SlackChannel::BattleBots.id() {
            info!("recognized message from {}", channel);

            if text.starts_with("!put ") {
                info!("matched !put");
                parse_put(message_standard)
            } else if text.starts_with("!last") {
                info!("matched !last");
                last_five(message_standard)
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

    let token = utils::get_slack_token_from_env_var();

    let mut handler = Handler;
    let r = RtmClient::login_and_run(&token, &mut handler);
    match r {
        Ok(_) => {}
        Err(err) => panic!("Error: {}", err),
    }
}
