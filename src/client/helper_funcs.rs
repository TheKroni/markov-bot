use crate::*;
use serenity::{
    builder::ParseValue,
    client::Context,
    model::{
        channel::{GuildChannel, Message},
        interactions::{
            application_command::{
                ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
            },
            message_component::ButtonStyle,
        },
    },
    prelude::Mentionable,
};

use super::tags::global_data::get_tag_response_channel_id_lock;

pub fn user_id_command(command: &ApplicationCommandInteraction) -> String {
    let options = command
        .data
        .options
        .get(0)
        .expect("Expected user option")
        .resolved
        .as_ref()
        .expect("Expected user object");
    if let ApplicationCommandInteractionDataOptionValue::User(user, _member) = options {
        format!("{}'s id is {}", user, user.id)
    } else {
        "Please provide a valid user".to_owned()
    }
}

/**
It first tries to send a message in the same channel.

If that fails then it sends the message to the tag response channel if one is set

If that fails then it iterates through every channel in the guild until it finds one it can send a message in
*/
pub async fn send_message_to_first_available_channel(ctx: &Context, msg: &Message, message: &str) {
    let bot_channels = get_tag_response_channel_id_lock(&ctx.data).await;
    let bot_channel_id = bot_channels.get(&msg.guild_id.expect("Couldn't get the guild id").0);

    if msg.channel_id.say(&ctx.http, message).await.is_err() {
        //try sending message to bot channel
        if let Some(channel_id) = bot_channel_id {
            let bot_channel = ctx.cache.guild_channel(*channel_id).await;
            if let Some(channel) = bot_channel {
                channel
                    .send_message(&ctx.http, |m| {
                        if rand::random::<f32>() < 0.05 {
                            m.components(|c| {
                                c.create_action_row(|a| {
                                    a.create_button(|b| {
                                        b.label("Stop pinging me")
                                            .style(ButtonStyle::Primary)
                                            .custom_id(ButtonIds::BlacklistMeFromTags)
                                    })
                                })
                            });
                        }
                        m.allowed_mentions(|m| m.parse(ParseValue::Users))
                            .content(msg.author.mention().to_string() + " " + message)
                    })
                    .await
                    .expect("Couldn't send message");
                return;
            }
        }

        //iterate until it manages to send a message
        let channels: Vec<GuildChannel> = msg
            .guild(&ctx.cache)
            .await
            .expect("Couldn't retrieve guild from cache")
            .channels
            .iter()
            .map(|(_, channel)| channel.clone())
            .collect();
        for channel in channels {
            match channel
                .id
                .send_message(&ctx.http, |m| {
                    m.components(|c| {
                        c.create_action_row(|a| {
                            a.create_button(|b| {
                                b.label("Stop pinging me")
                                    .style(ButtonStyle::Primary)
                                    .custom_id(ButtonIds::BlacklistMeFromTags)
                            })
                        })
                    })
                    .allowed_mentions(|m| m.parse(ParseValue::Users))
                    .content(msg.author.mention().to_string() + " " + message)
                })
                .await
            {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    }
}
