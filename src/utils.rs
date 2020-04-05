//! utility functions that don't belong anywhere else
use crate::SlackChannel;
use log::{debug, info};
use reqwest::blocking::Client;
use serde_json::Value;
use slack_api::reactions::AddRequest;
use slack_api::users::{InfoRequest, InfoResponse};

pub fn get_slack_token_from_env_var() -> String {
    // get bot token from environment variables
    let target_env_var = "SLACKBOT_TOKEN_SECRET";
    let mut api_key_json: String = String::new();
    for (k, v) in std::env::vars() {
        if k == target_env_var {
            api_key_json = v;
        }
    }

    if api_key_json.is_empty() {
        eprintln!(
            "no {} environment variable found!\nPlease set this env var and try again.",
            target_env_var
        );
        std::process::exit(1);
    }

    let slackbot_token_json: Value = serde_json::from_str(&api_key_json).unwrap();
    String::from(slackbot_token_json["SLACKBOT_TOKEN"].as_str().unwrap())
}

fn make_client() -> Client {
    slack_api::requests::default_client().unwrap()
}

pub fn bot_say(channel: SlackChannel, msg: &str) {
    let api_client = make_client();
    let token = get_slack_token_from_env_var();

    let chan_id = channel.id();
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

pub fn add_reaction(request: AddRequest) {
    info!("adding reaction");
    let api_client = make_client();
    let token = get_slack_token_from_env_var();
    let res = slack_api::reactions::add(&api_client, &token, &request);
    debug!("{:?}", res);
}

pub fn get_user_info(user_id: &str) -> Option<InfoResponse> {
    let api_client = make_client();
    let token = get_slack_token_from_env_var();
    let mut info_request = InfoRequest::default();
    info_request.user = user_id;
    if let Ok(user_info) = slack_api::users::info(&api_client, &token, &info_request) {
        Some(user_info)
    } else {
        None
    }
}

pub fn get_user_handle(user_id: &str) -> Option<String> {
    if let Some(info) = get_user_info(user_id) {
        info.user.unwrap().name
    } else {
        None
    }
}

pub fn get_user_real_name(user_id: &str) -> Option<String> {
    if let Some(info) = get_user_info(user_id) {
        let user_real_name = info.user.unwrap().real_name.unwrap();
        Some(user_real_name)
    } else {
        None
    }
}
