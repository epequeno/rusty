extern crate rss;
use rss::Channel;
use std::time::Duration;
use std::thread;
use slack::Sender;

pub fn read_feed(feed: &str, sender: Sender) {
  let mut channel = Channel::from_url(feed).unwrap();  
  let previous_title = channel.clone().into_items()[0].clone();
  let mut previous_title = previous_title.title().unwrap().to_string();
  loop {
    channel = Channel::from_url(feed).unwrap();  
    let latest = channel.into_items()[0].clone();
    let link = latest.clone();
    let link = link.link().unwrap();
    let title = latest.clone();
    let title = title.title().unwrap().clone();
    if title != previous_title {
      // we now have a different title than the last time we ran
      // we'll consider this evidence of an update to the feed.
      // C8EHWNKHV == #rust
      let msg = format!("<{}|{}>", link, title);
      let _ = sender.send_message("D8S4B7Q8H", &msg);
      previous_title = title.to_string();
    }
    thread::sleep(Duration::from_secs(300));
  }
}