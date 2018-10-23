extern crate openssl_probe;
extern crate slack;
extern crate rss;

use slack::{Event, EventHandler, Message, RtmClient};
mod reader;
use reader::read_feed;

struct Handler;

#[allow(unused_variables)]
impl EventHandler for Handler {
  fn on_event(&mut self, client: &RtmClient, event: Event) {
    println!("on_event(event: {:?})", event);

    match event.clone() {
      Event::Message(message) => self.handle_message(*message, client, &event),
      _ => return
    };
  }

  fn on_close(&mut self, client: &RtmClient) {}

  fn on_connect(&mut self, client: &RtmClient) {
    let feeds = vec![
      "https://blog.rust-lang.org/feed.xml",
      "https://newrustacean.com/feed.xml",
      "https://this-week-in-rust.org/rss.xml",
      "https://rusty-spike.blubrry.net/feed/podcast/",
    ];

    for feed in feeds {
      let sender = client.sender().clone();
      std::thread::spawn(move || {
        read_feed(feed, &sender);
      });
    }
  }
}

#[allow(unused_variables)]
impl Handler {
  fn handle_message(&mut self, message: Message, client: &RtmClient, event: &Event) {
    let message_standard = match message {
      Message::Standard(message_standard) => message_standard,
      _ => return
    };

    let channel: String = message_standard.channel.unwrap();
    let bot_id: &str = client.start_response().slf.as_ref().unwrap().id.as_ref().unwrap();
    let text: String = message_standard.text.unwrap();
    if text.contains(bot_id) {
      println!("is a mention");
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
    println!("no {} environment variable found!\nPlease set this env var and try again.", target_env_var);
    std::process::exit(1);
  }

  let mut handler = Handler;
  let r = RtmClient::login_and_run(&api_key, &mut handler);
  match r {
    Ok(_) => {}
    Err(err) => panic!("Error: {}", err)
  }
}
