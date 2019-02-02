//! rss and atom readers
//! useful for rss debug: http://lorem-rss.herokuapp.com/feed?unit=minute&interval=60
use crate::SlackChannel;
use atom_syndication::{Entry, Feed};
use linked_hash_set::LinkedHashSet;
use log::{debug, error};
use reqwest;
use rss::{Channel, Item};
use slack::Sender;
use std::thread;
use std::time::Duration;

fn get_titles(items: Vec<Item>) -> Vec<String> {
    let mut titles: Vec<String> = Vec::new();
    for item in items {
        titles.push(item.title().unwrap_or("none").into());
    }
    titles
}

#[derive(Clone)]
pub struct RssFeed {
    pub url: String,
    pub previous_titles: LinkedHashSet<String>,
    pub slack_channel: SlackChannel,
}

impl RssFeed {
    pub fn new(url: String) -> RssFeed {
        debug!("creating new rss feed with: {}", &url);
        let previous_titles: LinkedHashSet<String> = LinkedHashSet::new();
        RssFeed {
            url,
            previous_titles,
            slack_channel: SlackChannel::BattleBots,
        }
    }

    pub fn read(&self) -> Vec<Item> {
        debug!("reading rss feed: {}", self.url);
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

#[derive(Clone)]
pub struct AtomFeed {
    pub url: String,
    pub previous_titles: LinkedHashSet<String>,
    pub slack_channel: SlackChannel,
}

impl AtomFeed {
    pub fn new(url: String) -> AtomFeed {
        debug!("creating new atom feed with: {}", url);
        let previous_titles: LinkedHashSet<String> = LinkedHashSet::new();
        AtomFeed {
            url,
            previous_titles,
            slack_channel: SlackChannel::BattleBots,
        }
    }

    pub fn read(&self) -> Vec<Entry> {
        debug!("reading atom feed: {}", self.url);
        let res = reqwest::get(&self.url).unwrap().text().unwrap();
        let feed: Feed = res.parse().unwrap();
        feed.entries().to_vec()
    }
}

// READ LOOPS

pub fn read_rss_feed(mut feed: RssFeed, sender: Sender) {
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
            let popped = feed
                .previous_titles
                .pop_front()
                .unwrap_or_else(|| "none".into());
            debug!("popping previous title: {}", popped);
        }

        let items = feed.read();
        debug!("got {} items from {}", items.len(), feed.url);

        // find any new, unseen items
        let mut new_items: Vec<Item> = Vec::new();
        for item in items {
            let title = item.title().unwrap_or("none");
            if !feed.previous_titles.contains(title) {
                debug!("found new title: {}", title);
                new_items.push(item.clone());
                feed.previous_titles.insert(title.to_string());
            }
        }

        // send new items
        for item in new_items {
            let latest_title = item.title().unwrap_or("none");
            let link = item.link().unwrap_or("https://sanantoniodevs.com/");
            let msg = format!("<{}|{}>", link, latest_title);
            debug!("sending channel {}: {}", chan_id, msg);
            let _ = sender.send_message(&chan_id, &msg);
        }

        thread::sleep(sleep_duration);
    }
}

pub fn read_atom_feed(mut feed: AtomFeed, sender: Sender) {
    let sleep_duration = Duration::from_secs(300);
    let titles_to_retain = 200;

    // initial run
    let chan_id = feed.slack_channel.get_channel_id();
    let entries = feed.read();
    debug!("got {} items from {}", entries.len(), feed.url);
    for entry in entries {
        feed.previous_titles.insert(entry.title().into());
    }
    thread::sleep(sleep_duration);

    // main reader loop
    loop {
        // keep previously seen titles to a reasonable size
        while feed.previous_titles.len() > titles_to_retain {
            let popped = feed
                .previous_titles
                .pop_front()
                .unwrap_or_else(|| "none".into());
            debug!("popping previous title: {}", popped);
        }

        let entries = feed.read();
        debug!("got {} items from {}", entries.len(), feed.url);

        // find any new, unseen items
        let mut new_items: Vec<Entry> = Vec::new();
        for entry in entries {
            let title = entry.title();
            if !feed.previous_titles.contains(title) {
                debug!("found new title: {}", title);
                new_items.push(entry.clone());
                feed.previous_titles.insert(title.to_string());
            }
        }

        // send new items
        for item in new_items {
            let latest_title = item.title();
            let link = item.id();
            let msg = format!("<{}|{}>", link, latest_title);
            debug!("sending channel {}: {}", chan_id, msg);
            let _ = sender.send_message(&chan_id, &msg);
        }

        thread::sleep(sleep_duration);
    }
}
