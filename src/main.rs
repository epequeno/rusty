use slack::{Event, EventHandler, Message, RtmClient};
mod reader;
use env_logger;
use log::info;
use reader::{read_feed, Feed};

struct Handler;

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
        let rust_channel = String::from("C8EHWNKHV");
        let aws_channel = String::from("CA6MUA4LU");
        let k8s_channel = String::from("C91DM9Y6S");
        let rss_feeds = [
            (rust_channel.clone(), "https://blog.japaric.io/index.xml"),
            (rust_channel.clone(), "https://newrustacean.com/feed.xml"),
            (rust_channel.clone(), "https://nercury.github.io/feed.xml"),
            (rust_channel.clone(), "https://os.phil-opp.com/rss.xml"),
            (
                rust_channel.clone(),
                "https://this-week-in-rust.org/rss.xml",
            ),
            (
                rust_channel.clone(),
                "https://rusty-spike.blubrry.net/feed/podcast/",
            ),
            (aws_channel.clone(), "https://aws.amazon.com/new/feed/"),
            (k8s_channel.clone(), "https://kubernetes.io/feed.xml"),
            // (
            //     "C91DM9Y6S",
            //     "http://lorem-rss.herokuapp.com/feed?unit=minute&interval=60",
            // ),
        ];

        let atom_feeds = [
            ("C8EHWNKHV", "https://blog.rust-lang.org/feed.xml"),
            ("C6DTBQK4P", "http://feeds.feedburner.com/PythonInsider"),
        ];

        // let feeds = [("CA6MUA4LU", "https://blog.japaric.io/index.xml")];
        // let feeds = [(
        //     "CA6MUA4LU",
        //     "http://lorem-rss.herokuapp.com/feed?unit=minute",
        // )];

        for (channel, url) in rss_feeds.iter() {
            let sender = client.sender().clone();
            let mut feed = Feed::new(url.to_string());
            feed.slack_channel = channel.to_string();
            std::thread::spawn(move || {
                read_feed(feed, sender);
            });
        }
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
