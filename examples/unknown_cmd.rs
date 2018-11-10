extern crate futures;
extern crate telebot;

use telebot::RcBot;
use futures::stream::Stream;
use std::env;

// import all available functions
use telebot::functions::*;

fn main() {
    // Create the bot
    let bot = RcBot::new(&env::var("TELEGRAM_BOT_KEY").unwrap()).unwrap();

    // Every possible command is unknown
    let handle = bot.unknown_cmd().and_then(|(bot, msg)| bot.message(msg.chat.id, "Unknown command".into()).send());

    bot.register(handle);

    // Enter the main loop
    bot.run()
}
