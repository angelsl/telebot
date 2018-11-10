extern crate futures;
extern crate telebot;

use telebot::RcBot;
use futures::{Future, stream::Stream};
use std::env;

fn main() {
    // Create the bot
    let bot = RcBot::new(&env::var("TELEGRAM_BOT_KEY").unwrap()).unwrap();

    // Enter the main loop
    let stream = bot.get_stream().then::<_, Result<(), ()>>(|res| {
        if let Err(err) = res {
            eprintln!("Event loop shutdown:");
            for (i, cause) in err.iter_causes().enumerate() {
                eprintln!(" => {}: {}", i, cause);
            }
        }

        Ok(())
    });

    tokio::run(stream.into_future().then(|_| Ok(())));
}
