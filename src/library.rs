//! functions for use in #library
use crate::utils::{bot_say, get_user_real_name};
use crate::SlackChannel;
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use prettytable::{format, Cell, Row, Table};
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, PutItemInput, QueryInput};
use std::collections::HashMap;
use url::Url;

fn parse_slack_url(url: &str) -> &str {
    info!("parser got url: {}", url);
    if url.len() == 1 {
        return url;
    }

    if url.contains('|') {
        // when slack gets a bare url like example.com it attempts to present it as a full link
        // in the UI and modifies it to slacks syntax for a link. for example:
        // example.com becomes <http://example.com|example.com>
        let url_string_parts: Vec<&str> = url.split('|').collect();
        let url_string = url_string_parts[0];

        // ignore the leading angle bracket
        &url_string[1..]
    } else {
        // a url sent to slack as http://example.com will look like <http://example.com> by the time
        // the bot sees it. We'll keep everything except the leading and trailing angle brackets.
        &url[1..url.len() - 1]
    }
}

// put a record of who put which url into a DB.
fn put_url(url: &str, user: &str) {
    info!("got request to put record for user: {}, url: {}", user, url);
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

    item.insert(String::from("partition_key"), partition_key_value);
    item.insert(String::from("url"), url_value);
    item.insert(String::from("added_at"), added_at);
    item.insert(String::from("user"), user_val);

    let mut put_item_input = PutItemInput::default();
    put_item_input.table_name = String::from("library");
    put_item_input.item = item;
    debug!("{:?}", client.put_item(put_item_input).sync());
}

// take the string we get from slack and parse it so we can do actual work with it
pub fn parse_put(text: &str, user: &str) {
    // expected input is like: !put <url>
    let parts: Vec<&str> = text.split(' ').collect();
    if parts.len() != 2 {
        let msg = format!("got {} parts, expected 2", parts.len());
        error!("{}", msg);
        bot_say(SlackChannel::Library, &msg);
        return;
    }

    let input_string = parts[1];
    let url_string = parse_slack_url(input_string);

    if let Ok(parsed_url) = Url::parse(&url_string) {
        info!("success! parsed {} as url: {}", url_string, parsed_url);
        put_url(&parsed_url.as_str(), user);
    } else {
        let msg = format!("unable to parse as url: {}", input_string);
        error!("{}", msg);

        bot_say(SlackChannel::Library, &msg)
    }
}

// get the five most recent entries from the DB
pub fn last_five() {
    let client = DynamoDbClient::new(Region::UsEast1);

    // define the query
    let mut query_input = QueryInput::default();
    query_input.table_name = String::from("library");
    query_input.select = Some(String::from("ALL_ATTRIBUTES"));
    query_input.index_name = Some(String::from("id-added_at-index"));
    query_input.limit = Some(5);
    // sort in reverse order, where newest are listed first
    query_input.scan_index_forward = Some(false);
    query_input.key_condition_expression =
        Some(String::from("id = :partition AND added_at >= :t1"));

    let mut attr_values: HashMap<String, AttributeValue> = HashMap::new();

    let mut attr_value = AttributeValue::default();
    attr_value.s = Some(String::from("records"));
    attr_values.insert(String::from(":partition"), attr_value);

    // TODO: change to a dynamic time range, so maybe only look at records in the last 6mo? Really
    // depends on the frequency of use which is currently unknown.
    let mut attr_value = AttributeValue::default();
    attr_value.s = Some(String::from("2020"));
    attr_values.insert(String::from(":t1"), attr_value);

    query_input.expression_attribute_values = Some(attr_values);

    let query_output = client.query(query_input).sync().unwrap();
    debug!("{:?}", query_output);
    let items = query_output.items.unwrap();

    if items.is_empty() {
        let msg = String::from("no records found!");
        bot_say(SlackChannel::Library, &msg);
        return;
    }

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row!["user", "added at", "url"]);

    for item in items.iter() {
        let mut row: Vec<Cell> = Vec::new();

        for key in vec!["user", "added_at", "url"].iter() {
            let value = item.get(&(*key).to_string()).unwrap().s.as_ref().unwrap();

            if key == &"user" {
                let real_name = get_user_real_name(value).unwrap();
                row.push(Cell::new(&real_name));
            } else if key == &"added_at" {
                // stored in db as: 2020-02-29T03:22:22.728369605+00:00
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
    bot_say(SlackChannel::Library, &msg)
}
