//! functions for use in #library
use crate::{bot_say, SlackChannel};
use chrono::{DateTime, Utc};
use log::{error, info};
use prettytable::{Cell, Row, Table};
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, PutItemInput, ScanInput};
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

pub fn parse_put(text: &str, user: &str) {
    let parts: Vec<&str> = text.split(' ').collect();
    if parts.len() != 2 {
        error!("got {} parts, expected 2", parts.len());
        return;
    }

    let input_string = parts[1];
    let url_string = parse_slack_url(input_string);

    if let Ok(parsed_url) = Url::parse(&url_string) {
        info!("success parsing as url: {}", parsed_url);
        put_url(&parsed_url.as_str(), user);
    } else {
        let msg = format!("unable to parse as url: {}", input_string);
        error!("{}", msg);

        bot_say(SlackChannel::BattleBots, &msg)
    }
}

pub fn last_five() {
    let client = DynamoDbClient::new(Region::UsEast1);
    let mut scan_input = ScanInput::default();
    scan_input.table_name = String::from("library");
    scan_input.select = Some(String::from("ALL_ATTRIBUTES"));
    scan_input.limit = Some(5);

    let scan_output = client.scan(scan_input).sync().unwrap();
    let items = scan_output.items.unwrap();

    let mut table = Table::new();
    table.add_row(row!["user", "url", "added_at"]);

    for item in items.iter() {
        let mut row: Vec<Cell> = Vec::new();

        for key in vec!["user", "url", "added_at"].iter() {
            let value = item.get(key.clone()).unwrap().s.as_ref().unwrap();
            row.push(Cell::new(&value));
        }

        table.add_row(Row::new(row));
    }

    let msg = table.to_string();
    bot_say(SlackChannel::BattleBots, &msg)
}
