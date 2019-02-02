//! rss reader
//! useful for debug: http://lorem-rss.herokuapp.com/feed?unit=minute&interval=60
use crate::SlackChannel;
use linked_hash_set::LinkedHashSet;
use log::{debug, error};
use rss::{Channel, Item};
use slack::Sender;
use std::thread;
use std::time::Duration;

fn get_titles(items: Vec<Item>) -> Vec<String> {
    let mut titles: Vec<String> = Vec::new();
    for item in items {
        titles.push(item.title().unwrap().to_string());
    }
    titles
}

#[derive(Clone)]
pub struct Feed {
    pub url: String,
    pub previous_titles: LinkedHashSet<String>,
    pub slack_channel: SlackChannel,
}

impl Feed {
    pub fn new(url: String) -> Feed {
        debug!("creating new Feed with: {}", &url);
        let previous_titles: LinkedHashSet<String> = LinkedHashSet::new();
        Feed {
            url,
            previous_titles,
            slack_channel: SlackChannel::BattleBots,
        }
    }

    pub fn read(&self) -> Vec<Item> {
        debug!("reading feed: {}", self.url);
        let channel = Channel::from_url(&self.url);
        match channel {
            Ok(chan) => chan.into_items(),
            Err(e) => {
                error!("error with: {}: {}", self.url, e);
                Vec::new()
            }
        }
    }
}

pub fn read_feed(mut feed: Feed, sender: Sender) {
    let sleep_duration = Duration::from_secs(300);
    let titles_to_retain = 200;

    // initial run
    let chan_id = feed.slack_channel.get_channel_id();
    let items = feed.read();
    debug!("got {} items from {}", items.len(), feed.url);
    for title in get_titles(items) {
        feed.previous_titles.insert(title);
    }
    thread::sleep(sleep_duration);

    // main reader loop
    loop {
        // keep previously seen titles to a reasonable size
        while feed.previous_titles.len() > titles_to_retain {
            let popped = feed.previous_titles.pop_front().unwrap();
            debug!("popping previous title: {}", popped);
        }

        let items = feed.read();
        debug!("got {} items from {}", items.len(), feed.url);

        // find any new, unseen items
        let mut new_items: Vec<Item> = Vec::new();
        for item in items {
            let title = item.title().unwrap();
            if !feed.previous_titles.contains(title) {
                debug!("found new title: {}", title);
                new_items.push(item.clone());
                feed.previous_titles.insert(title.to_string());
            }
        }

        // send new items
        for item in new_items {
            let latest_title = item.title().unwrap();
            let link = item.link().unwrap();
            let msg = format!("<{}|{}>", link, latest_title);
            debug!("sending channel {}: {}", chan_id, msg);
            let _ = sender.send_message(&chan_id, &msg);
        }

        thread::sleep(sleep_duration);
    }
}
