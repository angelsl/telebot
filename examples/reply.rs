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

    // Register a reply command which answers a message
    let handle = bot.new_cmd("/reply").and_then(|(bot, msg)| {
        let mut text = msg.text.unwrap().clone();
        if text.is_empty() {
            text = "<empty>".into();
        }

        bot.message(msg.chat.id, text).send()
    });

    bot.register(handle);

    // Enter the main loop
    bot.run();
}
