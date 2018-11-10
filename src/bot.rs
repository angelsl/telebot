//! This is the actual Bot module. For ergonomic reasons there is a RcBot which uses the real bot
//! as an underlying field. You should always use RcBot.

use objects;
use functions::FunctionGetMe;
use failure::{Error, Fail, ResultExt};
use error::{ErrorKind, TelegramError};
use file::File;

use std::{str, collections::HashMap, sync::{Arc, RwLock, atomic::{Ordering, AtomicUsize}}};

use hyper::{Body, Client, Request, Uri, header::CONTENT_TYPE, client::{HttpConnector, ResponseFuture}};
use hyper_tls::HttpsConnector;
use hyper_multipart::client::multipart;
use serde_json::{self, value::Value};
use futures::{stream, Future, IntoFuture, Stream, sync::mpsc::{self, UnboundedSender}};

/// A clonable, single threaded bot
///
/// The outer API gets implemented on RcBot
#[derive(Clone)]
pub struct RcBot {
    pub inner: Arc<Bot>,
}

impl RcBot {
    pub fn new(key: &str) -> Result<RcBot, Error> {
        Ok(RcBot {
            inner: Arc::new(Bot::new(key)?),
        })
    }
}

/// The main bot structure
pub struct Bot {
    pub key: String,
    pub name: RwLock<Option<String>>,
    pub last_id: AtomicUsize,
    pub timeout: AtomicUsize,
    pub handlers: RwLock<HashMap<String, UnboundedSender<(RcBot, objects::Message)>>>,
    pub unknown_handler: RwLock<Option<UnboundedSender<(RcBot, objects::Message)>>>,
    pub client: Client<HttpsConnector<HttpConnector>, Body>
}

impl Bot {
    pub fn new(key: &str) -> Result<Bot, Error> {
        debug!("Create a new bot with the key {}", key);

        Ok(Bot {
            key: key.into(),
            name: RwLock::new(None),
            last_id: AtomicUsize::new(0),
            timeout: AtomicUsize::new(120),
            handlers: RwLock::new(HashMap::new()),
            unknown_handler: RwLock::new(None),
            client: Client::builder()
                .keep_alive(true)
                .keep_alive_timeout(None)
                .build(HttpsConnector::new(4)
                    .context(ErrorKind::HttpsInitializeError)?)
        })
    }

    /// Creates a new request and adds a JSON message to it. The returned Future contains a the
    /// reply as a string.  This method should be used if no file is added becontext a JSON msg is
    /// always compacter than a formdata one.
    pub fn fetch_json(
        &self,
        func: &'static str,
        msg: &str,
    ) -> impl Future<Item = String, Error = Error> {
        debug!("Send JSON {}: {}", func, msg);

        let request = self.build_json(func, String::from(msg));

        request
            .into_future()
            .and_then(|(client, request)| _fetch(client.request(request)))
    }

    /// Builds the HTTP header for a JSON request. The JSON is already converted to a str and is
    /// appended to the POST header.
    fn build_json(
        &self,
        func: &'static str,
        msg: String,
    ) -> Result<(Client<HttpsConnector<HttpConnector>, Body>, Request<Body>), Error> {
        let url: Result<Uri, _> =
            format!("https://api.telegram.org/bot{}/{}", self.key, func).parse();

        let req = Request::post(url.context(ErrorKind::Uri)?)
            .header(CONTENT_TYPE, "application/json")
            .body(msg.into())
            .context(ErrorKind::Hyper)?;

        Ok((self.client.clone(), req))
    }

    /// Creates a new request with some byte content (e.g. a file). The method properties have to be
    /// in the formdata setup and cannot be sent as JSON.
    pub fn fetch_formdata(
        &self,
        func: &'static str,
        msg: &Value,
        files: Vec<File>,
        kind: &str,
    ) -> impl Future<Item = String, Error = Error> {
        debug!("Send formdata {}: {}", func, msg.to_string());

        let request = self.build_formdata(func, msg, files, kind);

        request
            .into_future()
            .and_then(|(client, request)| _fetch(client.request(request)))
    }

    /// Builds the HTTP header for a formdata request. The file content is read and then append to
    /// the formdata. Each key-value pair has a own line.
    fn build_formdata(
        &self,
        func: &'static str,
        msg: &Value,
        files: Vec<File>,
        _kind: &str,
    ) -> Result<
        (
            Client<HttpsConnector<HttpConnector>, Body>,
            Request<Body>,
        ),
        Error,
    > {
        let url: Result<Uri, _> =
            format!("https://api.telegram.org/bot{}/{}", self.key, func).parse();

        let mut req_builder = Request::post(url.context(ErrorKind::Uri)?);
        let mut form = multipart::Form::default();

        let msg = msg.as_object().ok_or(ErrorKind::JsonNotMap)?;

        // add properties
        for (key, val) in msg.iter() {
            let val = match val {
                &Value::String(ref val) => format!("{}", val),
                etc => format!("{}", etc),
            };

            form.add_text(key, val.as_ref());
        }

        for file in files {
            match file {
                File::Memory { name, source } => {
                    form.add_reader_file(name.clone(), source, name);
                }
                File::Disk { path } => {
                    form.add_file(path.clone().file_name().unwrap().to_str().unwrap(), path).context(ErrorKind::NoFile)?;
                },
                _ => {}
            }
        }

        let req = form.set_body(&mut req_builder).context(ErrorKind::Hyper)?;

        Ok((self.client.clone(), req))
    }
}

/// Calls the Telegram API for the function and awaits the result. The result is then converted
/// to a String and returned in a Future.
pub fn _fetch(fut_res: ResponseFuture) -> impl Future<Item = String, Error = Error> {
    fut_res
        .and_then(move |res| res.into_body().concat2())
        .map_err(|e| Error::from(e.context(ErrorKind::Hyper)))
        .and_then(move |response_chunks| {
            let s = str::from_utf8(&response_chunks)?;

            debug!("Got a result from telegram: {}", s);
            // try to parse the result as a JSON and find the OK field.
            // If the ok field is true, then the string in "result" will be returned
            let req = serde_json::from_str::<Value>(&s).context(ErrorKind::JsonParse)?;

            let ok = req.get("ok")
                .and_then(Value::as_bool)
                .ok_or(ErrorKind::Json)?;

            if ok {
                if let Some(result) = req.get("result") {
                    return Ok(serde_json::to_string(result).context(ErrorKind::JsonSerialize)?);
                }
            }

            let e = match req.get("description").and_then(Value::as_str) {
                Some(err) => {
                    Error::from(TelegramError::new(err.into()).context(ErrorKind::Telegram))
                }
                None => Error::from(ErrorKind::Telegram),
            };

            Err(Error::from(e.context(ErrorKind::Telegram)))
        })
}

impl RcBot {
    /// Sets the timeout interval for long polling
    pub fn timeout(self, timeout: usize) -> RcBot {
        self.inner.timeout.store(timeout, Ordering::Release);

        self
    }

    /// Creates a new command and returns a stream which will yield a message when the command is send
    pub fn new_cmd(
        &self,
        cmd: &str,
    ) -> impl Stream<Item = (RcBot, objects::Message), Error = Error> {
        let (sender, receiver) = mpsc::unbounded();

        let cmd = if cmd.starts_with("/") {
            cmd.into()
        } else {
            format!("/{}", cmd)
        };

        if let Ok(mut handlers) = self.inner.handlers.write() {
            handlers.insert(cmd.into(), sender);
        } else {
            warn!("poisoned lock in telebot");
        }

        receiver.map_err(|_| Error::from(ErrorKind::Channel))
    }

    /// Returns a stream which will yield a message when none of previously registered commands matches
    pub fn unknown_cmd(&self) -> impl Stream<Item = (RcBot, objects::Message), Error = Error> {
        let (sender, receiver) = mpsc::unbounded();

        if let Ok(mut unknown_handler) = self.inner.unknown_handler.write() {
            *unknown_handler = Some(sender);
        } else {
            warn!("poisoned lock in telebot");
        }

        receiver.then(|x| x.map_err(|_| Error::from(ErrorKind::Channel)))
    }

    /// Register a new commnd
    pub fn register<T>(&self, hnd: T)
    where
        T: Stream + Send + 'static,
        <T as Stream>::Error: Send
    {
        tokio::spawn(
            hnd.for_each(|_| Ok(()))
                .into_future()
                .map(|_| ())
                .map_err(|_| ()),
        );
    }

    /// The main update loop.
    /// When an update is available the last_id will be updated and the message is filtered
    /// for commands
    /// The message is forwarded to the returned stream if no command was found
    pub fn get_stream<'a>(
        &'a self,
    ) -> impl Stream<Item = (RcBot, objects::Update), Error = Error> + 'static {
        use functions::*;
        let me = self.clone();
        let y = stream::unfold((), move |_| {
                Some(me.get_updates()
                    .offset(me.inner.last_id.load(Ordering::Acquire) as i64)
                    .timeout(me.inner.timeout.load(Ordering::Acquire) as i64)
                    .send().map(|x| (x, ())))
            }).map(|(_, x)| {
                stream::iter_result(
                    x.0
                        .into_iter()
                        .map(|x| Ok(x))
                        .collect::<Vec<Result<objects::Update, Error>>>(),
                )
            })
            .flatten();
        let me = self.clone();
        let y = y.and_then(move |x| {
                if me.inner.last_id.load(Ordering::Acquire) < x.update_id as usize + 1 {
                    me.inner.last_id.store(x.update_id as usize + 1, Ordering::Release);
                }

                Ok(x)
            });
        let me = self.clone();
        y.filter_map(move |mut val| {
                debug!("Got an update from Telegram: {:?}", val);

                let mut sndr: Option<UnboundedSender<(RcBot, objects::Message)>> = None;

                if let Some(ref mut message) = val.message {
                    if let Some(text) = message.text.clone() {
                        let mut content = text.split_whitespace();
                        if let Some(mut cmd) = content.next() {
                            if cmd.starts_with("/") {
                                if let Ok(guard) = me.inner.name.read() {
                                    if let Some(ref name) = *guard {
                                        if cmd.ends_with(name.as_str()) {
                                            cmd = cmd.rsplitn(2, '@').skip(1).next().unwrap();
                                        }
                                    }
                                }
                                if let Ok(guard) = me.inner.handlers.write() {
                                    if let Some(sender) = guard.get(cmd) {
                                        sndr = Some(sender.clone());
                                        message.text = text.get(cmd.len()..).map(|x| x.to_owned());
                                    }
                                } else if let Ok(guard) = me.inner.unknown_handler.write() {
                                    if let Some(ref sender) = *guard {
                                        sndr = Some(sender.clone());
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(sender) = sndr {
                    sender
                        .unbounded_send((me.clone(), val.message.unwrap()))
                        .unwrap_or_else(|e| error!("Error: {}", e));
                    return None;
                } else {
                    return Some((me.clone(), val));
                }
            })
    }

    pub fn resolve_name(&self) {
        // create a local copy of the bot to circumvent lifetime issues
        let bot = self.inner.clone();
        // create a new task which resolves the bot name and then set it in the struct
        let resolve_name = self.get_me().send()
            .map(move |user| {
                if let Some(name) = user.1.username {
                    if let Ok(mut myname) = bot.name.write() {
                        *myname = Some(format!("@{}", name));
                    } else {
                        warn!("poisoned lock in telebot");
                    }

                }
            })
            .map_err(|e| {
                eprintln!("Error: could not resolve the bot name!");

                for (i, cause) in e.iter_causes().enumerate() {
                    println!(" => {}: {}", i, cause);
                }
            });
        // spawn the task
        tokio::spawn(resolve_name.map_err(|_| ()));
    }

    /// helper function to start the event loop
    pub fn run(&self) {
        self.resolve_name();
        tokio::run(self.get_stream().for_each(|_| Ok(())).map_err(|_| ()).into_future())
    }
}
