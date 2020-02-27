//! functions for use in #library
use crate::SlackChannel;
use log::{error, info};
use slack::RtmClient;
use slack_api;
use url::Url;

fn parse_slack_url(url: &str) -> &str {
    info!("parser got url: {}", url);
    if url.len() == 1 {
        return url;
    }
    if url.contains("|") {
        let url_string: Vec<&str> = url.split("|").collect();
        return &url_string[0][1..];
    } else {
        // get rid of surrounding brackets
        return &url[1..url.len() - 1];
    }
}

pub fn parse_add(text: &str, client: &RtmClient) {
    let parts: Vec<&str> = text.split(" ").collect();
    if parts.len() != 2 {
        error!("got {} parts, expected 2", parts.len());
        return;
    }

    let input_string = parts[1];
    let url_string = parse_slack_url(input_string);

    if let Ok(parsed_url) = Url::parse(&url_string) {
        info!("success parsing as url: {}", parsed_url);
    } else {
        let msg = format!("unable to parse as url: {}", input_string);
        error!("{}", msg);
        let channel = SlackChannel::BattleBots.id();
        let bot_msg = format!("```{}```", msg);
        let _ = client.sender().send_message(channel, &bot_msg);
    }
}
