use handlebars::{
    Context, Handlebars, Helper, HelperResult, Output, RenderContext, RenderErrorReason,
};
use std::str::FromStr;

use crate::config::Channel;

use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    ClientBuilder,
};

fn json_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param = h.param(0).ok_or_else(|| {
        RenderErrorReason::ParamNotFoundForIndex("json helper: missing parameter", 0)
    })?;

    let value = param.value();
    let json_str = serde_json::to_string(value).map_err(|e| RenderErrorReason::SerdeError(e))?;

    let trimmed = if json_str.starts_with('"') && json_str.ends_with('"') {
        &json_str[1..json_str.len() - 1]
    } else {
        &json_str
    };

    out.write(trimmed)?;

    Ok(())
}

pub struct SmtpChannel {}

impl TryFrom<Channel> for SmtpChannel {
    type Error = bool;

    #[allow(unused)]
    fn try_from(value: Channel) -> Result<Self, Self::Error> {
        Err(false)
    }
}

#[allow(unused)]
#[derive(Debug)]
pub enum ParsingError {
    InvalidType,
    MissingOption(String),
    InvalidOptionType(String),
}

#[derive(Default, Clone)]
pub struct WebhookChannel<'a> {
    pub url: String,
    pub message: String,
    pub message_template: Handlebars<'a>,
    pub headers: Option<HeaderMap>,
}

macro_rules! get_nec_opt {
    ($conf:ident, $map:expr, $opt_name:tt, $type:ident) => {
        match $map.get(stringify!($opt_name)) {
            Some(v) => match v {
                toml::Value::$type(s) => $conf.$opt_name = s.clone(),
                _ => {
                    return Err(ParsingError::InvalidOptionType(format!(
                        "Expected {:?} for {}",
                        stringify!($type),
                        stringify!($opt_name)
                    )))
                }
            },
            None => {
                return Err(ParsingError::MissingOption(
                    stringify!($opt_name).to_string(),
                ));
            }
        }
    };
}

#[allow(unused)]
macro_rules! get_opt_opt {
    ($conf:ident, $map:expr, $opt_name:tt, $type:ident) => {
        match $map.get(stringify!($opt_name)) {
            Some(v) => match v {
                toml::Value::$type(s) => $conf.$opt_name = Some(s.clone()),
                _ => {
                    return Err(ParsingError::InvalidOptionType(format!(
                        "Expected {:?} for {}",
                        stringify!($type),
                        stringify!($opt_name)
                    )))
                }
            },
            None => $conf.$opt_name = None,
        }
    };
}

impl<'a> TryFrom<&Channel> for WebhookChannel<'a> {
    type Error = ParsingError;

    fn try_from(chan: &Channel) -> Result<Self, Self::Error> {
        if chan.chan_type != "webhook" {
            return Err(ParsingError::InvalidType);
        }

        let mut webhook = WebhookChannel {
            ..Default::default()
        };

        get_nec_opt!(webhook, chan.config, url, String);
        get_nec_opt!(webhook, chan.config, message, String);

        if chan.config.contains_key("headers") && chan.config["headers"].is_table() {
            let hdrs = chan.config["headers"].as_table().unwrap();

            let mut hm = HeaderMap::new();
            hdrs.iter().for_each(|(k, v)| {
                hm.insert(
                    HeaderName::from_str(k.as_str()).unwrap(),
                    HeaderValue::from_str(v.as_str().unwrap()).unwrap(),
                );
            });

            webhook.headers = Some(hm);
        }

        webhook.message_template = Handlebars::new();
        webhook
            .message_template
            .register_helper("json", Box::new(json_helper));

        webhook
            .message_template
            .register_template_string("t", &webhook.message)
            .unwrap();

        Ok(webhook)
    }
}

pub trait Notify {
    type NotifyError;

    async fn notify(&self, body: String) -> Result<(), Self::NotifyError>;
}

impl<'a> Notify for WebhookChannel<'a> {
    type NotifyError = reqwest::Error;

    async fn notify(&self, body: String) -> Result<(), Self::NotifyError> {
        let c = match &self.headers {
            Some(h) => ClientBuilder::new().default_headers(h.clone()),
            None => ClientBuilder::new(),
        };

        println!("Client: {:?}", c);

        let r = c.build().unwrap().post(&self.url).body(body).send().await?;

        println!(
            "Response status: {}, response text: {:?}",
            r.status(),
            r.text().await?
        );

        Ok(())
    }
}
