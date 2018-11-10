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

    let handle = bot.new_cmd("/send_self")
        .and_then(|(bot, msg)| {
            bot.document(msg.chat.id)
                .file("examples/send_self.rs")
                .send()
        })
        .map_err(|err| println!("{:?}", err.cause()));

    bot.register(handle);

    // enter the main loop
    bot.run()
}
