extern crate rss;
use rss::Channel;
use std::time::Duration;
use std::thread;
use slack::Sender;

pub fn read_feed(feed: &str, sender: &Sender) {
  let channel = match Channel::from_url(feed) {
    Ok(c) => c,
    Err(e) => {
      println!("{}: {}", feed, e);
      return
    }
  };

  let previous_title = match channel.clone().into_items().get(0) {
    Some(i) => i.clone(),
    None => {
      println!("no items found");
      return
    }
  };

  let mut previous_title = match previous_title.title() {
    Some(t) => t.to_string(),
    None => {
      println!("no title found");
      return
    }
  };

  loop {
    let channel = match Channel::from_url(feed) {
      Ok(c) => c,
      Err(e) => {
        println!("{}", e);
        return
      }
    };  
    
    let latest_item = match channel.into_items().get(0) {
      Some(i) => i.clone(),
      None => {
        println!("no items found");
        return
      }
    };

    let link = latest_item.clone();
    let link = match link.link() {
      Some(l) => l,
      None => {
        println!("no link found");
        return
      }
    };

    let title = latest_item.clone();
    let title = match title.title() {
      Some(t) => t,
      None => {
        println!("no title found");
        return
      }
    };
    
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