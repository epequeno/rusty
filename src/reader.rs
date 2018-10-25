extern crate rss;
use rss::Channel;
use std::time::Duration;
use std::thread;
use slack::Sender;

pub fn read_feed(feed: &str, sender: &Sender) {
  println!("start reading {}", feed);
  
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

  // really this is the starting title but we'll use the name later
  println!("starting title: {}", previous_title);

  loop {
    println!("reading {}", feed);
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

    let latest_title = latest_item.clone();
    let latest_title = match latest_title.title() {
      Some(t) => t,
      None => {
        println!("no title found");
        return
      }
    };
    
    println!("latest: {}", latest_title);
    println!("previous: {}", previous_title);
    if latest_title != previous_title {
      // we now have a different title than the last time we ran
      // we'll consider this evidence of an update to the feed.
      // C8EHWNKHV == #rust
      let msg = format!("<{}|{}>", link, latest_title);
      let _ = sender.send_message("CD31RPEFR", &msg);
      previous_title = latest_title.to_string();
    }
    println!("sleeping: {}", feed);
    thread::sleep(Duration::from_secs(300));
  }
}
