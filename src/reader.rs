//! rss and atom readers
//! useful for rss debug: http://lorem-rss.herokuapp.com/feed?unit=minute&interval=60
use crate::SlackChannel;
use atom_syndication::{Entry, Feed as AtomFeed};
use failure::Error;
use linked_hash_set::LinkedHashSet;
use log::{debug, error, info};
use reqwest;
use rss::{Channel, Item};
use slack_api;
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub struct Title(String);

struct ArticleUrl(String);

#[derive(Debug, Clone)]
pub struct FeedUrl(String);

pub struct Article {
    title: Title,
    url: ArticleUrl,
}

impl Default for Title {
    fn default() -> Title {
        let default_title = String::from("Default Title");
        Title(default_title)
    }
}

impl std::fmt::Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            Title(t) => write!(f, "{}", t),
        }
    }
}

impl Default for ArticleUrl {
    fn default() -> ArticleUrl {
        let default_url = String::from("https://satx.dev");
        ArticleUrl(default_url)
    }
}

impl std::fmt::Display for ArticleUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            ArticleUrl(u) => write!(f, "{}", u),
        }
    }
}

impl Default for FeedUrl {
    fn default() -> FeedUrl {
        let default_url = String::from("https://satx.dev");
        FeedUrl(default_url)
    }
}

impl std::fmt::Display for FeedUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            FeedUrl(u) => write!(f, "{}", u),
        }
    }
}

#[derive(Clone, Debug)]
pub enum FeedType {
    Rss,
    Atom,
    PythonInsider,
    TGIK,
    JonHoo,
}

pub trait ReadFeed {
    fn read(&self) -> Result<Vec<Article>, Error>;
}

fn read_rss(feed: &Feed) -> Result<Vec<Item>, rss::Error> {
    let FeedUrl(u) = &feed.url;
    let channel = Channel::from_url(&u);
    Ok(channel?.into_items())
}

fn get_atom_feed(url: &str) -> Result<String, reqwest::Error> {
    let timeout = Duration::from_secs(3);
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()?;
    let res = client.get(url).send()?;
    Ok(res.text()?)
}

fn read_atom(feed: &Feed) -> Result<Vec<Entry>, Error> {
    let FeedUrl(url) = &feed.url;
    let text = get_atom_feed(url)?;
    let atom: AtomFeed = text.parse()?;
    Ok(atom.entries().to_vec())
}

impl ReadFeed for Feed {
    fn read(&self) -> Result<Vec<Article>, Error> {
        debug!("reading {:?} feed: {}", self.feed_type, self.url);
        match &self.feed_type {
            FeedType::Rss => Ok(read_rss(&self)?
                .iter()
                .map(|item| Article {
                    url: ArticleUrl::from_str(item.link().unwrap_or_default()),
                    title: Title::from_str(item.title().unwrap_or_default()),
                })
                .collect()),

            FeedType::Atom => Ok(read_atom(&self)?
                .iter()
                .map(|entry| Article {
                    url: ArticleUrl::from_str(entry.id()),
                    title: Title::from_str(entry.title()),
                })
                .collect()),

            FeedType::PythonInsider => Ok(read_atom(&self)?
                .iter()
                .map(|entry| {
                    let url: String = entry
                        .links()
                        .iter()
                        .filter(|l| l.rel() == "alternate")
                        .map(|l| String::from(l.href()))
                        .collect();
                    Article {
                        url: ArticleUrl::from_str(&url),
                        title: Title::from_str(entry.title()),
                    }
                })
                .collect()),

            FeedType::TGIK => Ok(read_atom(&self)?
                .iter()
                .filter(|entry| entry.title().starts_with("TGI Kubernetes "))
                .map(|entry| {
                    let url = if let Some(link) = entry.links().first() {
                        ArticleUrl::from_str(link.href())
                    } else {
                        ArticleUrl::default()
                    };
                    Article {
                        url,
                        title: Title::from_str(entry.title()),
                    }
                })
                .collect()),

            FeedType::JonHoo => Ok(read_atom(&self)?
                .iter()
                .map(|entry| {
                    let url = if let Some(link) = entry.links().first() {
                        ArticleUrl::from_str(link.href())
                    } else {
                        ArticleUrl::default()
                    };
                    Article {
                        url,
                        title: Title::from_str(entry.title()),
                    }
                })
                .collect()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Feed {
    pub url: FeedUrl,
    pub feed_type: FeedType,
    pub previous_titles: LinkedHashSet<String>,
    pub channel: SlackChannel,
}

impl Feed {
    pub fn new(url: &str, feed_type: FeedType, channel: SlackChannel) -> Feed {
        Feed {
            url: FeedUrl::from_str(url),
            feed_type,
            previous_titles: LinkedHashSet::new(),
            channel,
        }
    }
}

impl ArticleUrl {
    fn from_str(s: &str) -> ArticleUrl {
        ArticleUrl(String::from(s))
    }
}

impl Title {
    fn from_str(s: &str) -> Title {
        Title(String::from(s))
    }
}

impl FeedUrl {
    pub fn from_str(s: &str) -> FeedUrl {
        FeedUrl(String::from(s))
    }
}

pub fn read_feeds() {
    let sleep_duration = Duration::from_secs(300);
    let titles_to_retain = 200;
    let token: String = std::env::vars()
        .filter(|(k, _)| k == "SLACKBOT_TOKEN")
        .map(|(_, v)| v)
        .collect();
    let client = slack_api::requests::default_client().unwrap();

    let rss_feeds = vec![
        ("https://blog.japaric.io/index.xml", SlackChannel::Rust),
        ("https://newrustacean.com/feed.xml", SlackChannel::Rust),
        ("https://nercury.github.io/feed.xml", SlackChannel::Rust),
        ("https://os.phil-opp.com/rss.xml", SlackChannel::Rust),
        ("https://this-week-in-rust.org/rss.xml", SlackChannel::Rust),
        (
            "https://rusty-spike.blubrry.net/feed/podcast/",
            SlackChannel::Rust,
        ),
        (
            "http://feeds.feedburner.com/AmazonWebServicesBlog",
            SlackChannel::Aws,
        ),
        ("https://kubernetes.io/feed.xml", SlackChannel::Kubernetes),
    ];

    let rss_feeds: Vec<Feed> = rss_feeds
        .iter()
        .map(|(url, chan)| Feed::new(url, FeedType::Rss, chan.clone()))
        .collect();

    let atom_feeds = vec![("https://blog.rust-lang.org/feed.xml", SlackChannel::Rust)];

    let atom_feeds: Vec<Feed> = atom_feeds
        .iter()
        .map(|(url, chan)| Feed::new(url, FeedType::Atom, chan.clone()))
        .collect();

    let mut all_feeds = Vec::new();
    all_feeds.extend(rss_feeds);
    all_feeds.extend(atom_feeds);
    all_feeds.extend(vec![
        Feed::new(
            "https://www.youtube.com/feeds/videos.xml?channel_id=UCjQU5ZI2mHswy7OOsii_URg",
            FeedType::TGIK,
            SlackChannel::BattleBots,
        ),
        Feed::new(
            "https://www.youtube.com/feeds/videos.xml?channel_id=UC_iD0xppBwwsrM9DegC5cQQ",
            FeedType::JonHoo,
            SlackChannel::BattleBots,
        ),
        Feed::new(
            "http://feeds.feedburner.com/PythonInsider",
            FeedType::PythonInsider,
            SlackChannel::Python,
        ),
    ]);

    // main loop
    loop {
        for feed in &mut all_feeds {
            // initial run
            if feed.previous_titles.is_empty() {
                match feed.read() {
                    Ok(articles) => {
                        info!(
                            "got {} articles from {:?} {}",
                            articles.len(),
                            feed.feed_type,
                            feed.url
                        );
                        let titles = articles.iter().map(|article| {
                            let Title(t) = &article.title;
                            t
                        });
                        for title in titles {
                            feed.previous_titles.insert(title.clone());
                        }
                    }
                    Err(e) => error!("{}", e),
                }
                continue;
            }

            while feed.previous_titles.len() > titles_to_retain {
                info!("popping: {:?}", feed.previous_titles.pop_front());
            }

            let chan_id = feed.channel.id();
            match feed.read() {
                Ok(articles) => {
                    info!(
                        "got {} articles from {:?} {}",
                        &articles.len(),
                        feed.feed_type,
                        feed.url
                    );
                    for article in articles {
                        let Title(title) = &article.title;

                        if !feed.previous_titles.contains(title) {
                            info!("found new title: {}", title);
                            feed.previous_titles.insert(title.to_string());

                            let text = format!("<{}|{}>", article.url, article.title);
                            info!("sending channel {}: {}", &chan_id, &text);
                            let mut msg = slack_api::chat::PostMessageRequest::default();
                            msg.channel = &chan_id;
                            msg.text = &text;
                            msg.as_user = Some(true);
                            debug!("{:?}", slack_api::chat::post_message(&client, &token, &msg));
                        }
                    }
                }
                Err(e) => error!("{}", e),
            }
        }
        thread::sleep(sleep_duration);
    }
}
