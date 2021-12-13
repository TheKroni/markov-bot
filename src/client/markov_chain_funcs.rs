use crate::*;
use markov_strings::Markov;
use regex::{Captures, Regex};
use serenity::{
    client::Context,
    model::{channel::Message, prelude::User},
};
use std::error::Error;
use std::{fs, u64};

pub async fn should_add_message_to_markov_file(msg: &Message, ctx: &Context) {
    if msg
        .channel_id
        .to_channel(&ctx.http)
        .await
        .unwrap()
        .guild()
        .is_some()
    {
        {
            let markov_blacklisted_users = get_markov_blacklisted_users_lock(&ctx.data).await;
            let markov_blacklisted_channels = get_markov_blacklisted_channels_lock(&ctx.data).await;

            if !markov_blacklisted_channels.contains(&msg.channel_id.0)
                && !markov_blacklisted_users.contains(&msg.author.id.0)
                && !msg.mentions_me(&ctx.http).await.unwrap()
                && msg.content.split(' ').count() >= 5
            {
                let re = Regex::new(r#"(?:(?:https?|ftp)://|\b(?:[a-z\d]+\.))(?:(?:[^\s()<>]+|\((?:[^\s()<>]+|(?:\([^\s()<>]+\)))?\))+(?:\((?:[^\s()<>]+|(?:\(?:[^\s()<>]+\)))?\)|[^\s`!()\[\]{};:'".,<>?«»“”‘’]))?"#).unwrap();
                let mut str = re.replace_all(&msg.content, "").into_owned();
                while str.ends_with(' ') {
                    str.pop();
                }
                let filtered_message = filter_message_for_markov_file(str, msg);
                //msg.reply(&ctx.http, &filtered_message).await.unwrap();
                append_to_markov_file(&filtered_message);
            }
        }
    }
}

pub async fn send_markov_text(ctx: &Context, msg: &Message) {
    let markov_lock = get_markov_chain_lock(&ctx.data).await;

    let markov_chain = markov_lock.read().await;

    match markov_chain.generate() {
        Ok(markov_result) => {
            let mut message = markov_result.text;
            if cfg!(debug_assertions) {
                message += " --debug";
            }
            msg.channel_id.say(&ctx.http, message).await.unwrap();
        }
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Try again later.")
                .await
                .unwrap();
        }
    };
}
/// Gets the [`Markov`] data set from `markov export.json`.
/// 
/// This is faster than getting the data set from `markov data set.txt` so it's useful for testing purposes
pub fn init_markov_debug() -> Result<Markov, Box<dyn Error>> {
    let mut markov: Markov = serde_json::from_str(&fs::read_to_string(create_file_if_missing(
        MARKOV_EXPORT_PATH,
        &serde_json::to_string(&Markov::new().export())?,
    )?)?)?;
    markov.set_max_tries(200);
    markov.set_filter(|r| {
        if r.text.split(' ').count() >= 5 && r.refs.len() >= 2 {
            return true;
        }
        false
    });
    Ok(markov)
}

/// Gets the [`Markov`] data set from `markov data set.txt` and returns the initialized markov chain
pub fn init_markov() -> Result<Markov, Box<dyn Error>> {
    let mut markov_chain = Markov::new();
    markov_chain.set_state_size(3).unwrap(); // Will never fail
    markov_chain.set_max_tries(200);
    markov_chain.set_filter(|r| {
        if r.text.split(' ').count() >= 5 && r.refs.len() >= 2 {
            return true;
        }
        false
    });
    let input_data = import_chain_from_file()?;
    markov_chain.add_to_corpus(input_data);
    Ok(markov_chain)
}

pub fn filter_message_for_markov_file(str: String, msg: &Message) -> String {
    let mut filtered_message = str;
    //THIS IS GONNA BE A PAIN IN THE ASS
    let user_regex = Regex::new(r"<@!?(\d+)>").unwrap();

    let regexes_to_replace_with_whitespace: Vec<Regex> = vec![
        Regex::new(r"<:?(\w+:)(\d+)>").unwrap(),  //emote regex
        Regex::new(r"<a:?(\w+:)(\d+)>").unwrap(), //animated emote regex
        Regex::new(r#"[,.!"\#$()=?*<>{}\[\]\\\|Łł@*;:+~ˇ^˘°˛`´˝]"#).unwrap(), //non alphanumeric regex
        Regex::new(r"^(\d{18})$").unwrap(), //remaining numbers from users regex
        Regex::new(r"\n").unwrap(),         //line feed regex
        Regex::new(r"[ ]{3}|[ ]{2}").unwrap(), //double and triple whitespace regex
        Regex::new(r"<@&(\d+)>").unwrap(),  // role regex
    ];

    let upper_case_regex = Regex::new(r"[A-Z][a-z0-9_-]{1,}").unwrap();

    loop {
        let mut number_of_matches: u16 = 0;

        while user_regex.is_match(&filtered_message) {
            number_of_matches += 1;

            filtered_message = user_regex
                .replace(&filtered_message, |caps: &Captures| {
                    let mut user_id = String::new();

                    for char in caps[0].chars() {
                        if char.is_digit(10) {
                            user_id += &char.to_string();
                        }
                    }
                    let user_id = user_id.parse::<u64>().unwrap();
                    let user = &msg
                        .mentions
                        .iter()
                        .find(|&user| user.id.0 == user_id)
                        .unwrap()
                        .name;
                    " ".to_owned() + user + " "
                })
                .into_owned();
        }
        for regex in &regexes_to_replace_with_whitespace {
            while regex.is_match(&filtered_message) {
                number_of_matches += 1;
                filtered_message = regex.replace_all(&filtered_message, " ").into_owned();
            }
        }
        while upper_case_regex.is_match(&filtered_message) {
            number_of_matches += 1;
            filtered_message = upper_case_regex
                .replace(&filtered_message, |caps: &Captures| caps[0].to_lowercase())
                .into_owned();
        }
        if number_of_matches == 0 {
            break;
        }
    }

    return filtered_message.trim().to_owned();
}

pub async fn blacklist_user_command(msg: &Message, ctx: &Context) -> String {
    let user = match get_first_mentioned_user(msg) {
        Some(returned_user) => returned_user,
        None => {
            return "Please specify a user".to_owned();
        }
    };
    add_or_remove_user_from_markov_blacklist(user, ctx).await
}

pub async fn blacklisted_command(ctx: &Context) -> String {
    let mut blacklisted_usernames = Vec::new();
    let blacklisted_users = get_markov_blacklisted_users_lock(&ctx.data).await;

    for user_id in blacklisted_users.iter() {
        blacklisted_usernames.push(ctx.http.get_user(*user_id).await.unwrap().name);
    }

    if blacklisted_usernames.is_empty() {
        return "Currently there are no blacklisted users".to_owned();
    }

    let mut message = String::from("Blacklisted users: ");
    for user_name in blacklisted_usernames {
        message += &(user_name + ", ");
    }
    message.pop();
    message.pop();
    message
}

pub async fn add_or_remove_user_from_markov_blacklist(user: &User, ctx: &Context) -> String {
    let blacklisted_users = get_markov_blacklisted_users_lock(&ctx.data).await;

    if blacklisted_users.contains(&user.id.0) {
        blacklisted_users.remove(&user.id.0);
        match save_markov_blacklisted_users(&*blacklisted_users) {
            Ok(_) => "Removed ".to_owned() + &user.name + " from the list of blacklisted users",
            Err(_) => "Couldn't remove the user from the file".to_owned(),
        }
    } else {
        blacklisted_users.insert(user.id.0);
        match save_markov_blacklisted_users(&*blacklisted_users) {
            Ok(_) => "Added ".to_owned() + &user.name + " to the list of blacklisted users",
            Err(_) => "Couldn't add the user to the file".to_owned(),
        }
    }
}
