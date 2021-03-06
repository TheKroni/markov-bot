use std::str::FromStr;

use super::{
    helper_funcs::{ping_command, user_id_command},
    tags::{
        blacklist_user_from_tags_command, create_tag, list, remove_tag, set_tag_response_channel, commands::TagCommandBuilder,
    }, voice::commands::VoiceCommandBuilder,
};
use crate::{global_data, markov, voice, GuildId};
use serenity::{
    client::Context,
    model::{prelude::{interaction::application_command::ApplicationCommandInteraction, command::Command}},
    model::application::command::CommandOptionType,
};
use strum_macros::{Display, EnumString};

/// All the slash commands the bot has implemented
#[allow(non_camel_case_types)]
#[derive(Display, EnumString)]
pub enum UserCommand {
    ping,
    id,
    #[strum(serialize = "blacklisted-data")]
    blacklisteddata,
    #[strum(serialize = "stop-saving-my-messages")]
    stopsavingmymessages,
    #[strum(serialize = "continue-saving-my-messages")]
    continuesavingmymessages,
    #[strum(serialize = "create-tag")]
    createtag,
    #[strum(serialize = "remove-tag")]
    removetag,
    tags,
    #[strum(serialize = "blacklist-me-from-tags")]
    blacklistmefromtags,
    #[strum(serialize = "set-tag-response-channel")]
    settagresponsechannel,
    help,
    version,

    // =====VOICE=====
    play,
    skip,
    stop,
    playing,
    queue,
    #[strum(serialize = "loop")]
    loop_song,
    #[strum(serialize = "swap-songs")]
    swap_songs,
}

/// Check which slash command was triggered, call the appropriate function and return a response to the user
pub async fn command_responses(command: &ApplicationCommandInteraction, ctx: Context) {
    let user = &command.user;

    match UserCommand::from_str(&command.data.name) {
        Ok(user_command) => match user_command {
            UserCommand::ping => ping_command(ctx, command).await,
            UserCommand::id => user_id_command(ctx, command).await,
            UserCommand::blacklisteddata => markov::blacklisted_users(ctx, command).await,
            UserCommand::stopsavingmymessages => {
                markov::add_user_to_blacklist(user, &ctx, command).await;
            }
            UserCommand::createtag => create_tag(&ctx, command).await,
            UserCommand::removetag => remove_tag(&ctx, command).await,
            UserCommand::tags => list(&ctx, command).await,
            UserCommand::blacklistmefromtags => {
                blacklist_user_from_tags_command(&ctx, user, command).await;
            }

            UserCommand::settagresponsechannel => set_tag_response_channel(&ctx, command).await,
            UserCommand::help => command
                .create_interaction_response(ctx.http, |r| {
                    r.interaction_response_data(|d| d.content(global_data::HELP_MESSAGE))
                })
                .await
                .expect("Error creating interaction response"),
                UserCommand::version => command
                .create_interaction_response(ctx.http, |r| {
                    r.interaction_response_data(|d| {
                        d.content("My current version is ".to_owned() + env!("CARGO_PKG_VERSION"))
                    })
                })
                .await
                .expect("Error creating interaction response"),
                UserCommand::continuesavingmymessages => {
                markov::remove_user_from_blacklist(user, &ctx, command).await;
            }

            // ===== VOICE =====
            UserCommand::play => voice::play(&ctx, command).await,
            UserCommand::skip => voice::skip(&ctx, command).await,
            UserCommand::stop => voice::stop(&ctx, command).await,
            UserCommand::playing => voice::playing(&ctx, command).await,
            UserCommand::queue => voice::queue(&ctx, command).await,
            UserCommand::loop_song => voice::loop_song(&ctx, command).await,
            UserCommand::swap_songs => voice::swap_songs(&ctx, command).await,
        },
        Err(why) => {
            eprintln!("Cannot respond to slash command {why}");
        }
    };
}

/// Create the slash commands
pub async fn create_global_commands(ctx: &Context) {
    Command::set_global_application_commands(&ctx.http, |commands| {
        commands
            .create_application_command(|command| {
                command.name(UserCommand::ping).description("A ping command")
            })
            .create_application_command(|command| {
                command
                    .name(UserCommand::id)
                    .description("Get a user id")
                    .create_option(|option| {
                        option
                            .name("id")
                            .description("The user to lookup")
                            .kind(CommandOptionType::User)
                            .required(true)
                    })
            })
            .create_application_command(|command| {
                command.name(UserCommand::blacklisteddata).description(
                    "Get the list of users who's messages aren't being saved",
                )
            })
            .create_application_command(|command| {
                command.name(UserCommand::stopsavingmymessages).description(
                    "Blacklist yourself if you don't want me to save and learn from your messages",
                )
            })
            .create_application_command(|command| {
                command.name(UserCommand::continuesavingmymessages).description(
                    "Remove yourself from the blacklist if you want me to save and learn from your messages",
                )
            })
            .create_application_command(|command| {
                command
                    .name(UserCommand::help)
                    .description("Information about my commands")
            })
            .create_application_command(|command| {
                command
                    .name(UserCommand::version)
                    .description("My current version")
            })
            .create_voice_commands()
            .create_tag_commands() 
    })
    .await
    .expect("Couldn't create global slash commands");
}

/// For testing purposes. Creates commands for a specific guild
pub async fn create_test_commands(ctx: &Context) {
    let testing_guild = std::env::var("TESTING_GUILD_ID")
        .expect("Expected a TESTING_GUILD_ID in the environment")
        .parse()
        .expect("Couldn't parse the TESTING_GUILD_ID");

    GuildId(testing_guild)
        .set_application_commands(&ctx.http, |commands| commands)
        .await
        .expect("Couldn't create guild test commands");
}
