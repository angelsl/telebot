extern crate futures;
extern crate telebot;
extern crate env_logger;

use telebot::RcBot;
use futures::{Future, stream::Stream};
use std::env;

fn main() {
    env_logger::init();

    // Create the bot
    let bot = RcBot::new(&env::var("TELEGRAM_BOT_KEY").unwrap()).unwrap();

    let stream = bot.get_stream().then::<_, Result<(), ()>>(|res| {
        match res {
            Ok((_, msg)) => println!("Received: {:#?}", msg),
            Err(err) => {
                eprintln!("Event loop shutdown:");
                for (i, cause) in err.iter_causes().enumerate() {
                    eprintln!(" => {}: {}", i, cause);
                }
            }
        };

        Ok(())
    });

    // enter the main loop
    tokio::run(stream.into_future().then(|_| Ok(())));
}
