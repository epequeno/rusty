//! functions for use in #library
use crate::{bot_say, SlackChannel};
use chrono::{DateTime, Utc};
use log::{error, info};
use prettytable::{format, Cell, Row, Table};
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, PutItemInput, QueryInput};
use slack_api::users::InfoRequest;
use std::collections::HashMap;
use url::Url;

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

    let mut partition_key_value = AttributeValue::default();
    partition_key_value.s = Some(String::from("records"));

    let utc: DateTime<Utc> = Utc::now();
    let mut added_at = AttributeValue::default();
    let timestamp = utc.to_rfc3339();
    added_at.s = Some(timestamp);

    let mut user_val = AttributeValue::default();
    user_val.s = Some(user.to_string());

    item.insert(String::from("id"), partition_key_value);
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

fn get_user_info(user_id: &str) -> Option<String> {
    let api_client = slack_api::requests::default_client().unwrap();
    let token: String = std::env::vars()
        .filter(|(k, _)| k == "SLACKBOT_TOKEN")
        .map(|(_, v)| v)
        .collect();
    let mut info_request = InfoRequest::default();
    info_request.user = user_id;
    if let Ok(res) = slack_api::users::info(&api_client, &token, &info_request) {
        let user_real_name = res.user.unwrap().real_name.unwrap();
        Some(user_real_name)
    } else {
        None
    }
}

pub fn last_five() {
    let client = DynamoDbClient::new(Region::UsEast1);
    let mut query_input = QueryInput::default();
    query_input.table_name = String::from("library");
    query_input.select = Some(String::from("ALL_ATTRIBUTES"));
    query_input.index_name = Some(String::from("id-added_at-index"));
    query_input.limit = Some(5);
    query_input.scan_index_forward = Some(false);
    query_input.key_condition_expression =
        Some(String::from("id = :partition AND added_at >= :t1"));

    let mut attr_values: HashMap<String, AttributeValue> = HashMap::new();

    let mut attr_value = AttributeValue::default();
    attr_value.s = Some(String::from("records"));
    attr_values.insert(String::from(":partition"), attr_value);

    let mut attr_value = AttributeValue::default();
    attr_value.s = Some(String::from("2020"));
    attr_values.insert(String::from(":t1"), attr_value);

    query_input.expression_attribute_values = Some(attr_values);

    let query_output = client.query(query_input).sync().unwrap();
    info!("{:?}", query_output);
    let items = query_output.items.unwrap();

    if items.is_empty() {
        let msg = String::from("no records found!");
        bot_say(SlackChannel::BattleBots, &msg);
        return;
    }

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row!["user", "added_at", "url"]);

    for item in items.iter() {
        let mut row: Vec<Cell> = Vec::new();

        for key in vec!["user", "added_at", "url"].iter() {
            let value = item.get(&(*key).to_string()).unwrap().s.as_ref().unwrap();
            if key == &"user" {
                let real_name = get_user_info(value).unwrap();
                row.push(Cell::new(&real_name));
            } else if key == &"added_at" {
                let parts: Vec<&str> = value.split('.').collect();
                let date_time = parts[0];
                let parts: Vec<&str> = date_time.split('T').collect();
                let (date, time) = (parts[0], parts[1]);
                let date_time = format!("{} {} UTC", date, time);
                row.push(Cell::new(&date_time));
            } else {
                row.push(Cell::new(&value));
            }
        }

        table.add_row(Row::new(row));
    }

    let msg = table.to_string();
    bot_say(SlackChannel::BattleBots, &msg)
}
