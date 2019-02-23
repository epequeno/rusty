//! rss and atom readers
//! useful for rss debug: http://lorem-rss.herokuapp.com/feed?unit=minute&interval=60
use crate::SlackChannel;
use atom_syndication::Feed as AtomFeed;
use linked_hash_set::LinkedHashSet;
use log::debug;
use reqwest;
use rss::Channel;
use slack::Sender;
use std::thread;
use std::time::Duration;

pub trait Feed {
    fn read(&self) -> Vec<Article>;
    fn get_feed_info(&self) -> FeedInfo;
    fn set_feed_info(&mut self, feed_info: FeedInfo);
    fn get_previous_titles(&self) -> LinkedHashSet<String>;
    fn insert_title(&mut self, title: String);
    fn pop(&mut self);
}

#[derive(Clone, Debug)]
pub struct Article {
    title: String,
    url: String,
}

#[derive(Clone)]
pub struct FeedInfo {
    pub url: Option<String>,
    previous_titles: LinkedHashSet<String>,
}

#[derive(Clone)]
pub struct Rss {
    feed_info: FeedInfo,
}

#[derive(Clone)]
pub struct Atom {
    feed_info: FeedInfo,
}

#[derive(Clone)]
pub struct PythonInsider {
    feed_info: FeedInfo,
}

impl FeedInfo {
    pub fn new() -> FeedInfo {
        FeedInfo {
            url: None,
            previous_titles: LinkedHashSet::new(),
        }
    }
}

impl Rss {
    pub fn new() -> Rss {
        let default_info = FeedInfo::new();
        Rss {
            feed_info: default_info,
        }
    }
}

impl Atom {
    pub fn new() -> Atom {
        let default_info = FeedInfo::new();
        Atom {
            feed_info: default_info,
        }
    }
}

impl PythonInsider {
    pub fn new() -> PythonInsider {
        let default_info = FeedInfo::new();
        PythonInsider {
            feed_info: default_info,
        }
    }
}

impl Feed for Rss {
    fn read(&self) -> Vec<Article> {
        match &self.feed_info.url {
            Some(u) => {
                let channel = Channel::from_url(u);
                match channel {
                    Ok(chan) => chan
                        .into_items()
                        .iter()
                        .map(|item| Article {
                            url: item.link().unwrap_or("https://sanantoniodevs.com/").into(),
                            title: item.title().unwrap_or("none").into(),
                        })
                        .collect(),
                    Err(_) => Vec::new(),
                }
            }
            None => Vec::new(),
        }
    }

    fn get_feed_info(&self) -> FeedInfo {
        self.feed_info.clone()
    }

    fn set_feed_info(&mut self, feed_info: FeedInfo) {
        self.feed_info = feed_info;
    }

    fn get_previous_titles(&self) -> LinkedHashSet<String> {
        self.feed_info.clone().previous_titles
    }

    fn insert_title(&mut self, title: String) {
        self.feed_info.previous_titles.insert(title);
    }

    fn pop(&mut self) {
        self.feed_info.previous_titles.pop_front();
    }
}

impl Feed for Atom {
    fn read(&self) -> Vec<Article> {
        match &self.feed_info.url {
            Some(u) => {
                let result = reqwest::get(u).unwrap().text().unwrap();
                let feed: AtomFeed = result.parse().unwrap();
                feed.entries()
                    .to_vec()
                    .iter()
                    .map(|entry| Article {
                        url: entry.id().into(),
                        title: entry.title().into(),
                    })
                    .collect()
            }
            None => Vec::new(),
        }
    }

    fn get_feed_info(&self) -> FeedInfo {
        self.feed_info.clone()
    }

    fn set_feed_info(&mut self, feed_info: FeedInfo) {
        self.feed_info = feed_info;
    }

    fn get_previous_titles(&self) -> LinkedHashSet<String> {
        self.feed_info.clone().previous_titles
    }

    fn insert_title(&mut self, title: String) {
        self.feed_info.previous_titles.insert(title);
    }

    fn pop(&mut self) {
        self.feed_info.previous_titles.pop_front();
    }
}

impl Feed for PythonInsider {
    fn read(&self) -> Vec<Article> {
        match &self.feed_info.url {
            Some(u) => {
                let result = reqwest::get(u).unwrap().text().unwrap();
                let feed: AtomFeed = result.parse().unwrap();
                feed.entries()
                    .to_vec()
                    .iter()
                    .map(|entry| {
                        let mut url = String::new();
                        for link in entry.links() {
                            if link.rel() == "alternate" {
                                url = link.href().into();
                            };
                        }
                        Article {
                            url,
                            title: entry.title().into(),
                        }
                    })
                    .collect()
            }
            None => Vec::new(),
        }
    }

    fn get_feed_info(&self) -> FeedInfo {
        self.feed_info.clone()
    }

    fn set_feed_info(&mut self, feed_info: FeedInfo) {
        self.feed_info = feed_info;
    }

    fn get_previous_titles(&self) -> LinkedHashSet<String> {
        self.feed_info.clone().previous_titles
    }

    fn insert_title(&mut self, title: String) {
        self.feed_info.previous_titles.insert(title);
    }

    fn pop(&mut self) {
        self.feed_info.previous_titles.pop_front();
    }
}

pub fn read_feed<T: Feed>(mut feed: T, channel: SlackChannel, sender: Sender) {
    let sleep_duration = Duration::from_secs(120);
    let titles_to_retain = 200;

    // initial run
    let chan_id = channel.channel_id();
    let articles = feed.read();
    debug!(
        "got {} articles from {}",
        articles.len(),
        feed.get_feed_info().url.unwrap()
    );
    for article in articles {
        feed.insert_title(article.title);
    }
    thread::sleep(sleep_duration);

    // main reader loop
    loop {
        // keep previously seen titles to a reasonable size
        while feed.get_previous_titles().len() > titles_to_retain {
            let mut previous_titles = feed.get_previous_titles();
            let popped = previous_titles.pop_front().unwrap_or_else(|| "none".into());
            debug!("popping previous title: {}", popped);
            feed.pop();
        }

        let articles = feed.read();
        debug!(
            "got {} articles from {}",
            articles.len(),
            feed.get_feed_info().url.unwrap_or_else(|| "err".into())
        );

        // find any new, unseen items
        let mut new_articles: Vec<Article> = Vec::new();
        for article in articles {
            let title = article.title.clone();
            if !feed.get_previous_titles().contains(&title) {
                debug!("found new title: {}", title);
                new_articles.push(article.clone());
                feed.insert_title(title);
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
