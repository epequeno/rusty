extern crate rss;
use rss::Channel;
use std::time::Duration;
use std::thread;

pub fn read_feed() {
  let url = "https://blog.rust-lang.org/feed.xml";
  let mut channel = Channel::from_url(&url).unwrap();  
  let mut last_known = channel.into_items().pop();
  loop {
    thread::sleep(Duration::from_secs(10));
    channel = Channel::from_url(&url).unwrap();  
    let latest = channel.into_items().pop();
    if latest.clone() != last_known {
      let desc = latest.clone().unwrap();
      let desc = desc.description().unwrap();
      let link = latest.clone().unwrap();
      let link = link.link().unwrap();
      println!("{} {}",  &desc, &link);
      last_known = latest.clone();
    } else {
      println!("no change");
    }
  }
}