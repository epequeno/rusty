extern crate rss;
use rss::Channel;
use std::time::Duration;
use std::thread;
use slack::Sender;

pub fn read_feed(feed: &str, sender: Sender) {
  let mut channel = Channel::from_url(feed).unwrap();  
  let mut last_known = channel.into_items()[0].clone();
  loop {
    channel = Channel::from_url(feed).unwrap();  
    let latest = channel.into_items()[0].clone();
    if latest.clone() != last_known {
      let title = latest.clone();
      let title = title.title().unwrap();
      let desc = latest.clone();
      let desc = desc.description().unwrap();
      let link = latest.clone();
      let link = link.link().unwrap();
      let msg = format!("{} {} {}", title, desc, link);
      // C8EHWNKHV == #rust
      let _ = sender.send_message("C8EHWNKHV", &msg);
      last_known = latest;
    }
    thread::sleep(Duration::from_secs(3600));
  }
}