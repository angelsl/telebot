extern crate futures;
extern crate telebot;

use telebot::RcBot;
use futures::{IntoFuture, Future, stream::Stream};
use std::env;

use telebot::functions::*;
use telebot::objects::*;

fn main() {
    // Create the bot
    let bot = RcBot::new(&env::var("TELEGRAM_BOT_KEY").unwrap()).unwrap();

    let stream = bot.get_stream()
        .filter_map(|(bot, msg)| msg.inline_query.map(|query| (bot, query)))
        .and_then(|(bot, query)| {
            let result = vec![
                    InlineQueryResultArticle::new(
                        "Test".into(),
                        input_message_content::Text::new("This is a test".into()).into(),
                    ).reply_markup(InlineKeyboardMarkup::new(vec![
                        vec![
                            InlineKeyboardButton::new("Wikipedia".into())
                                .url("http://wikipedia.org"),
                        ],
                    ]))
                    .into()
            ];

            bot.answer_inline_query(query.id, result)
                .is_personal(true)
                .send()
        });

    // enter the main loop
    tokio::run(stream.for_each(|_| Ok(())).map_err(|_| ()).into_future());
}
