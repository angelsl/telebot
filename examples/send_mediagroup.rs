extern crate futures;
extern crate telebot;

use telebot::{RcBot, file::File};
use futures::stream::Stream;
use std::env;

// import all available functions
use telebot::functions::*;

fn main() {
    // Create the bot
    let bot = RcBot::new(&env::var("TELEGRAM_BOT_KEY").unwrap()).unwrap();

    let handle = bot.new_cmd("/send_mediagroup")
        .and_then(|(bot, msg)| {
            bot.mediagroup(msg.chat.id)
                .file(File::Url("https://upload.wikimedia.org/wikipedia/commons/f/f4/Honeycrisp.jpg".into()))
                .file(File::Url("https://upload.wikimedia.org/wikipedia/en/3/3e/Pooh_Shepard1928.jpg".into()))
                .file("examples/bee.jpg")
                .send()
        })
        .map_err(|err| println!("{:?}", err.cause()));

    bot.register(handle);

    // enter the main loop
    bot.run();
}
