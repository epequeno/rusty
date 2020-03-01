//! utility functions that don't belong anywhere else
use crate::SlackChannel;
use log::info;
use slack_api::users::InfoRequest;

fn get_slack_token_from_env_var() -> String {
    std::env::vars()
        .filter(|(k, _)| k == "SLACKBOT_TOKEN")
        .map(|(_, v)| v)
        .collect()
}

pub fn bot_say(channel: SlackChannel, msg: &str) {
    let api_client = slack_api::requests::default_client().unwrap();
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

// get user_real_name
pub fn get_user_real_name(user_id: &str) -> Option<String> {
    let api_client = slack_api::requests::default_client().unwrap();
    let token = get_slack_token_from_env_var();
    let mut info_request = InfoRequest::default();
    info_request.user = user_id;
    if let Ok(res) = slack_api::users::info(&api_client, &token, &info_request) {
        let user_real_name = res.user.unwrap().real_name.unwrap();
        Some(user_real_name)
    } else {
        None
    }
}
