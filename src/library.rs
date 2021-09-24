//! functions for use in #library
use crate::utils::{add_reaction, bot_say, get_user_handle, get_user_real_name};
use crate::SlackChannel;
use chrono::offset::TimeZone;
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use prettytable::{format, Cell, Row, Table};
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, PutItemInput, QueryInput};
use slack_api::{reactions::AddRequest, MessageStandard, Timestamp};
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
    let url_value = AttributeValue {
        s: Some(url.to_string()),
        ..Default::default()
    };

    let partition_key_value = AttributeValue {
        s: Some(String::from("records")),
        ..Default::default()
    };

    let utc: DateTime<Utc> = Utc::now();
    let timestamp = utc.timestamp().to_string();
    let timestamp_attr_value = AttributeValue {
        s: Some(timestamp),
        ..Default::default()
    };

    let user_val = AttributeValue {
        s: Some(user.to_string()),
        ..Default::default()
    };

    let user_real_name_val = AttributeValue {
        s: Some(get_user_real_name(user).unwrap_or_default()),
        ..Default::default()
    };

    let user_handle = AttributeValue {
        s: Some(get_user_handle(user).unwrap_or_default()),
        ..Default::default()
    };

    item.insert(String::from("partition_key"), partition_key_value);
    item.insert(String::from("url"), url_value);
    item.insert(String::from("timestamp"), timestamp_attr_value);
    item.insert(String::from("user"), user_val);
    item.insert(String::from("real_name"), user_real_name_val);
    item.insert(String::from("handle"), user_handle);

    let put_item_input = PutItemInput {
        table_name: String::from("library"),
        item,
        ..Default::default()
    };

    let res = client.put_item(put_item_input).sync();
    debug!("{:?}", res);
    res
}

// take the string we get from slack and parse it so we can do actual work with it
pub fn parse_put(message: MessageStandard) {
    // expected input is like: !put <url>
    let timestamp: Timestamp = message.ts.unwrap();
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
        let add_request = AddRequest {
            channel: Some(&channel),
            timestamp: Some(timestamp),
            name: if put_url(&parsed_url.as_str(), &user).is_ok() {
                "heavy_check_mark"
            } else {
                "x"
            },
            ..Default::default()
        };
        add_reaction(add_request)
    } else {
        let msg = format!("unable to parse as url: {}", input_string);
        error!("{}", msg);

        bot_say(SlackChannel::Library, &msg)
    }
}

// get the five most recent entries from the DB
pub fn last_five(message: MessageStandard) {
    let client = DynamoDbClient::new(Region::UsEast1);

    let mut attr_values: HashMap<String, AttributeValue> = HashMap::new();
    let mut attr_names: HashMap<String, String> = HashMap::new();
    attr_names.insert(String::from("#timestamp"), String::from("timestamp"));

    let attr_value = AttributeValue {
        s: Some(String::from("records")),
        ..Default::default()
    };
    attr_values.insert(String::from(":partition"), attr_value);

    // TODO: change to a dynamic time range, so maybe only look at records in the last 6mo? Really
    // depends on the frequency of use which is currently unknown.
    let attr_value = AttributeValue {
        s: Some(String::from("1577836800")),
        ..Default::default()
    };
    attr_values.insert(String::from(":t1"), attr_value);

    // define the query
    let query_input = QueryInput {
        table_name: String::from("library"),
        select: Some(String::from("ALL_ATTRIBUTES")),
        index_name: Some(String::from("partition_key-timestamp-index")),
        limit: Some(5),
        // sort in reverse order, where newest are listed first
        scan_index_forward: Some(false),
        key_condition_expression: Some(String::from(
            "partition_key = :partition AND #timestamp >= :t1",
        )),
        expression_attribute_names: Some(attr_names),
        expression_attribute_values: Some(attr_values),
        ..Default::default()
    };

    let query_output = client.query(query_input).sync().unwrap();
    debug!("{:?}", query_output);
    let items = query_output.items.unwrap();

    let channel: String = message.channel.unwrap();

    for chan in &[SlackChannel::BotSpam, SlackChannel::Library] {
        if channel == chan.to_string() {
            if items.is_empty() {
                let msg = String::from("no records found!");
                bot_say(chan.clone(), &msg);
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
            bot_say(chan.clone(), &msg)
        }
    }
}
