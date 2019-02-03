//! rss and atom readers
//! useful for rss debug: http://lorem-rss.herokuapp.com/feed?unit=minute&interval=60
use crate::SlackChannel;
use atom_syndication::Feed as AtomFeed;
use linked_hash_set::LinkedHashSet;
use log::{debug, error};
use reqwest;
use rss::Channel;
use slack::Sender;
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub struct Article {
    title: String,
    url: String,
}

#[derive(Clone)]
pub enum FeedType {
    Rss,
    Atom,
}

#[derive(Clone)]
pub struct Feed {
    pub url: String,
    pub feed_type: FeedType,
    pub previous_titles: LinkedHashSet<String>,
    pub slack_channel: SlackChannel,
}

impl Feed {
    pub fn new(feed_type: FeedType, url: String) -> Feed {
        let t = match feed_type {
            FeedType::Rss => "rss",
            FeedType::Atom => "atom",
        };
        debug!("creating new {} feed with: {}", t, &url);
        let previous_titles: LinkedHashSet<String> = LinkedHashSet::new();
        Feed {
            url,
            feed_type,
            previous_titles,
            slack_channel: SlackChannel::BattleBots,
        }
    }

    pub fn read(&self) -> Vec<Article> {
        match self.feed_type {
            FeedType::Rss => {
                debug!("reading rss feed: {}", self.url);
                let channel = Channel::from_url(&self.url);
                match channel {
                    Ok(chan) => chan
                        .into_items()
                        .iter()
                        .map(|item| Article {
                            url: item.link().unwrap_or("https://sanantoniodevs.com/").into(),
                            title: item.title().unwrap_or("none").into(),
                        })
                        .collect(),
                    Err(e) => {
                        error!("error with: {}: {}", self.url, e);
                        Vec::new()
                    }
                }
            }
            FeedType::Atom => {
                debug!("reading atom feed: {}", self.url);
                let res = reqwest::get(&self.url).unwrap().text().unwrap();
                let feed: AtomFeed = res.parse().unwrap();
                feed.entries()
                    .to_vec()
                    .iter()
                    .map(|entry| Article {
                        url: entry.title().into(),
                        title: entry.id().into(),
                    })
                    .collect()
            }
        }
    }
}

// READ LOOP
pub fn read_feed(mut feed: Feed, sender: Sender) {
    let sleep_duration = Duration::from_secs(300);
    let titles_to_retain = 200;

    // initial run
    let chan_id = feed.slack_channel.get_channel_id();
    let articles = feed.read();
    debug!("got {} articles from {}", articles.len(), feed.url);
    for article in articles {
        feed.previous_titles.insert(article.title);
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

        let articles = feed.read();
        debug!("got {} articles from {}", articles.len(), feed.url);

        // find any new, unseen items
        let mut new_articles: Vec<Article> = Vec::new();
        for article in articles {
            let title = article.title.clone();
            if !feed.previous_titles.contains(&title) {
                debug!("found new title: {}", title);
                new_articles.push(article.clone());
                feed.previous_titles.insert(title);
            }
        }

        // send new items
        for article in new_articles {
            let msg = format!("<{}|{}>", article.url, article.title);
            debug!("sending channel {}: {}", chan_id, msg);
            let _ = sender.send_message(&chan_id, &msg);
        }

        thread::sleep(sleep_duration);
    }
}
