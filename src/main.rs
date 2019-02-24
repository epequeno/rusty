use slack::{Event, EventHandler, Message, RtmClient};
mod reader;
use env_logger;
use log::info;
use reader::*;

struct Handler;

#[derive(Clone)]
pub enum SlackChannel {
    Aws,
    Rust,
    Kubernetes,
    Python,
    BattleBots,
}

impl SlackChannel {
    pub fn channel_id(&self) -> String {
        match self {
            SlackChannel::Aws => "CA6MUA4LU",
            SlackChannel::Rust => "C8EHWNKHV",
            SlackChannel::Kubernetes => "C91DM9Y6S",
            SlackChannel::Python => "C6DTBQK4P",
            SlackChannel::BattleBots => "CD31RPEFR",
        }
        .into()
    }
}

fn start_readers(client: &RtmClient) {
    // RSS
    let rss_feeds = [
        (SlackChannel::Rust, "https://blog.japaric.io/index.xml"),
        (SlackChannel::Rust, "https://newrustacean.com/feed.xml"),
        (SlackChannel::Rust, "https://nercury.github.io/feed.xml"),
        (SlackChannel::Rust, "https://os.phil-opp.com/rss.xml"),
        (SlackChannel::Rust, "https://this-week-in-rust.org/rss.xml"),
        (
            SlackChannel::Rust,
            "https://rusty-spike.blubrry.net/feed/podcast/",
        ),
        (SlackChannel::Aws, "https://aws.amazon.com/new/feed/"),
        (SlackChannel::Kubernetes, "https://kubernetes.io/feed.xml"),
    ];

    for (channel, url) in rss_feeds.iter() {
        let sender = client.sender().clone();
        let chan = channel.clone();
        let url = Some(url.to_string());
        std::thread::spawn(move || {
            let mut feed = Rss::new();
            feed.info = FeedInfo::new();
            feed.info.url = url;
            read_feed(feed, chan, sender);
        });
    }

    // ATOM
    let atom_feeds = [(SlackChannel::Rust, "https://blog.rust-lang.org/feed.xml")];

    for (channel, url) in atom_feeds.iter() {
        let sender = client.sender().clone();
        let chan = channel.clone();
        let url = Some(url.to_string());
        std::thread::spawn(move || {
            let mut feed = Atom::new();
            feed.info = FeedInfo::new();
            feed.info.url = url;
            read_feed(feed, chan, sender);
        });
    }

    // YouTube
    let sender = client.sender().clone();
    let mut feed = TGIK::new();
    feed.info.url =
        Some("https://www.youtube.com/feeds/videos.xml?channel_id=UCjQU5ZI2mHswy7OOsii_URg".into());
    std::thread::spawn(move || {
        read_feed(feed, SlackChannel::BattleBots, sender);
    });

    let sender = client.sender().clone();
    let mut feed = JonHoo::new();
    feed.info.url =
        Some("https://www.youtube.com/feeds/videos.xml?channel_id=UC_iD0xppBwwsrM9DegC5cQQ".into());
    std::thread::spawn(move || {
        read_feed(feed, SlackChannel::BattleBots, sender);
    });

    // PythonInsider
    let sender = client.sender().clone();
    let mut feed = PythonInsider::new();
    feed.info = FeedInfo::new();
    feed.info.url = Some("http://feeds.feedburner.com/PythonInsider".to_string());
    std::thread::spawn(move || {
        read_feed(feed, SlackChannel::Python, sender);
    });
}

#[allow(unused_variables)]
impl EventHandler for Handler {
    fn on_event(&mut self, client: &RtmClient, event: Event) {
        info!("on_event(event: {:?})", event);

        match event.clone() {
            Event::Message(message) => self.handle_message(*message, client, &event),
            _ => return,
        };
    }

    fn on_close(&mut self, client: &RtmClient) {}

    fn on_connect(&mut self, client: &RtmClient) {
        start_readers(client);
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
        let bot_id: &str = client
            .start_response()
            .slf
            .as_ref()
            .unwrap()
            .id
            .as_ref()
            .unwrap();
        let text: String = message_standard.text.unwrap();
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
    let target_env_var = "SLACKBOT_TOKEN";
    let mut api_key: String = "".to_string();
    for (k, v) in std::env::vars() {
        if k == target_env_var {
            api_key = v;
        }
    }

    if api_key.is_empty() {
        eprintln!(
            "no {} environment variable found!\nPlease set this env var and try again.",
            target_env_var
        );
        std::process::exit(1);
    }

    let mut handler = Handler;
    let r = RtmClient::login_and_run(&api_key, &mut handler);
    match r {
        Ok(_) => {}
        Err(err) => panic!("Error: {}", err),
    }
}
