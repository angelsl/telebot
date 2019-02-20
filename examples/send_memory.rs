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

    let text = r"
Dearest creature in creation,
Study English pronunciation.
I will teach you in my verse
Sounds like corpse, corps, horse, and worse.
I will keep you, Suzy, busy,
Make your head with heat grow dizzy.
Tear in eye, your dress will tear.
So shall I! Oh hear my prayer.

Just compare heart, beard, and heard,
Dies and diet, lord and word,
Sword and sward, retain and Britain.
(Mind the latter, how it's written.)
Now I surely will not plague you
With such words as plaque and ague.
But be careful how you speak:
Say break and steak, but bleak and streak;
Cloven, oven, how and low,
Script, receipt, show, poem, and toe.

Hear me say, devoid of trickery,
Daughter, laughter, and Terpsichore,
Typhoid, measles, topsails, aisles,
Exiles, similes, and reviles;
Scholar, vicar, and cigar,
Solar, mica, war and far;
One, anemone, Balmoral,
Kitchen, lichen, laundry, laurel;
Gertrude, German, wind and mind,
Scene, Melpomene, mankind.

...";

    let handle = bot.new_cmd("/send").and_then(move |(bot, msg)| {
        bot.document(msg.chat.id)
            .file(("poem.txt", text.as_bytes()))
            .caption("The Chaos")
            .send()
    });

    bot.register(handle);

    // enter the main loop
    bot.run()
}
