//! functions for use in #library
use crate::utils::{add_reaction, bot_say, get_user_real_name};
use crate::SlackChannel;
use chrono::offset::TimeZone;
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use prettytable::{format, Cell, Row, Table};
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, PutItemInput, QueryInput};
use slack_api::reactions::AddRequest;
use slack_api::MessageStandard;
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
fn put_url(
    url: &str,
    user: &str,
) -> Result<rusoto_dynamodb::PutItemOutput, rusoto_core::RusotoError<rusoto_dynamodb::PutItemError>>
{
    info!("got request to put record for user: {}, url: {}", user, url);
    let client = DynamoDbClient::new(Region::UsEast1);

    let mut item: HashMap<String, AttributeValue> = HashMap::new();
    let mut url_value = AttributeValue::default();
    url_value.s = Some(url.to_string());

    let mut partition_key_value = AttributeValue::default();
    partition_key_value.s = Some(String::from("records"));

    let utc: DateTime<Utc> = Utc::now();
    let mut timestamp_attr_value = AttributeValue::default();
    let timestamp = utc.timestamp().to_string();
    timestamp_attr_value.s = Some(timestamp);

    let mut user_val = AttributeValue::default();
    user_val.s = Some(user.to_string());

    let mut user_real_name_val = AttributeValue::default();
    if let Some(name) = get_user_real_name(user) {
        user_real_name_val.s = Some(name);
    } else {
        user_real_name_val.s = Some(String::new());
    }

    item.insert(String::from("partition_key"), partition_key_value);
    item.insert(String::from("url"), url_value);
    item.insert(String::from("timestamp"), timestamp_attr_value);
    item.insert(String::from("user"), user_val);
    item.insert(String::from("real_name"), user_real_name_val);

    let mut put_item_input = PutItemInput::default();
    put_item_input.table_name = String::from("library");
    put_item_input.item = item;
    let res = client.put_item(put_item_input).sync();
    debug!("{:?}", res);
    res
}

// take the string we get from slack and parse it so we can do actual work with it
pub fn parse_put(message: MessageStandard) {
    // expected input is like: !put <url>
    let timestamp: String = message.ts.unwrap();
    let text: String = message.text.unwrap();
    let user: String = message.user.unwrap();
    let channel: String = message.channel.unwrap();

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
        let mut add_request = AddRequest::default();
        add_request.channel = Some(&channel);
        add_request.timestamp = Some(&timestamp);

        match put_url(&parsed_url.as_str(), &user) {
            Ok(_) => {
                add_request.name = "heavy_check_mark";
                add_reaction(add_request)
            }
            Err(_) => {
                add_request.name = "x";
                add_reaction(add_request)
            }
        }
    } else {
        let msg = format!("unable to parse as url: {}", input_string);
        error!("{}", msg);

        bot_say(SlackChannel::Library, &msg)
    }
}

// get the five most recent entries from the DB
pub fn last_five(slack_channel: SlackChannel) {
    let client = DynamoDbClient::new(Region::UsEast1);

    // define the query
    let mut query_input = QueryInput::default();
    query_input.table_name = String::from("library");
    query_input.select = Some(String::from("ALL_ATTRIBUTES"));
    query_input.index_name = Some(String::from("partition_key-timestamp-index"));
    query_input.limit = Some(5);
    // sort in reverse order, where newest are listed first
    query_input.scan_index_forward = Some(false);
    query_input.key_condition_expression = Some(String::from(
        "partition_key = :partition AND #timestamp >= :t1",
    ));

    let mut attr_values: HashMap<String, AttributeValue> = HashMap::new();

    let mut attr_names: HashMap<String, String> = HashMap::new();
    attr_names.insert(String::from("#timestamp"), String::from("timestamp"));

    let mut attr_value = AttributeValue::default();
    attr_value.s = Some(String::from("records"));
    attr_values.insert(String::from(":partition"), attr_value);

    // TODO: change to a dynamic time range, so maybe only look at records in the last 6mo? Really
    // depends on the frequency of use which is currently unknown.
    let mut attr_value = AttributeValue::default();
    attr_value.s = Some(String::from("1577836800"));
    attr_values.insert(String::from(":t1"), attr_value);

    query_input.expression_attribute_names = Some(attr_names);
    query_input.expression_attribute_values = Some(attr_values);

    let query_output = client.query(query_input).sync().unwrap();
    debug!("{:?}", query_output);
    let items = query_output.items.unwrap();

    if items.is_empty() {
        let msg = String::from("no records found!");
        bot_say(slack_channel, &msg);
        return;
    }

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row!["user", "timestamp", "url"]);

    for item in items.iter() {
        let mut row: Vec<Cell> = Vec::new();

        for key in vec!["real_name", "timestamp", "url"].iter() {
            let value = item.get(&(*key).to_string()).unwrap().s.as_ref().unwrap();
            if key == &"timestamp" {
                let timestamp_int = value.parse::<i64>().unwrap();
                let dt = format!("{}", Utc.timestamp(timestamp_int, 0));
                row.push(Cell::new(&dt));
            } else {
                row.push(Cell::new(&value));
            }
        }

        table.add_row(Row::new(row));
    }

    let msg = table.to_string();
    bot_say(slack_channel, &msg)
}
