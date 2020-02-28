//! functions for use in #library
use crate::SlackChannel;
use chrono::{DateTime, Utc};
use log::{error, info};
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, PutItemInput};
use slack_api;
use std::collections::HashMap;
use url::Url;
use uuid::Uuid;

fn parse_slack_url(url: &str) -> &str {
    info!("parser got url: {}", url);
    if url.len() == 1 {
        return url;
    }

    if url.contains('|') {
        let url_string: Vec<&str> = url.split('|').collect();
        &url_string[0][1..]
    } else {
        // get rid of surrounding brackets
        &url[1..url.len() - 1]
    }
}

fn put_url(url: &str, user: &str) {
    let client = DynamoDbClient::new(Region::UsEast1);

    let mut item: HashMap<String, AttributeValue> = HashMap::new();
    let mut url_value = AttributeValue::default();
    url_value.s = Some(url.to_string());

    let mut id_value = AttributeValue::default();
    id_value.s = Some(Uuid::new_v4().to_string());

    let utc: DateTime<Utc> = Utc::now();
    let mut added_at = AttributeValue::default();
    added_at.s = Some(utc.to_rfc3339());

    let mut user_val = AttributeValue::default();
    user_val.s = Some(user.to_string());

    item.insert(String::from("id"), id_value);
    item.insert(String::from("url"), url_value);
    item.insert(String::from("added_at"), added_at);
    item.insert(String::from("user"), user_val);

    let mut put_item_input = PutItemInput::default();
    put_item_input.table_name = String::from("library");
    put_item_input.item = item;
    info!("{:?}", client.put_item(put_item_input).sync());
}

pub fn parse_add(text: &str, user: &str) {
    let parts: Vec<&str> = text.split(' ').collect();
    if parts.len() != 2 {
        error!("got {} parts, expected 2", parts.len());
        return;
    }
    let api_client = slack_api::requests::default_client().unwrap();
    let input_string = parts[1];
    let url_string = parse_slack_url(input_string);

    if let Ok(parsed_url) = Url::parse(&url_string) {
        info!("success parsing as url: {}", parsed_url);
        put_url(&parsed_url.as_str(), user);
    } else {
        let msg = format!("unable to parse as url: {}", input_string);
        error!("{}", msg);

        let token: String = std::env::vars()
            .filter(|(k, _)| k == "SLACKBOT_TOKEN")
            .map(|(_, v)| v)
            .collect();
        let chan_id = SlackChannel::BattleBots.id();
        let bot_msg = format!("```{}```", msg);

        let mut msg = slack_api::chat::PostMessageRequest::default();
        msg.channel = &chan_id;
        msg.text = &bot_msg;
        msg.as_user = Some(true);

        info!(
            "{:?}",
            slack_api::chat::post_message(&api_client, &token, &msg)
        );
    }
}
