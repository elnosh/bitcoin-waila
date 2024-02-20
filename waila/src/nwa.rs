use core::fmt;
use itertools::Itertools;
use nostr::key::PublicKey;
use nostr::nips::nip47::{Error, Method};
use nostr::prelude::url::form_urlencoded::byte_serialize;
use nostr::Url;
use std::borrow::Cow;
use std::str::FromStr;

fn url_encode<T>(data: T) -> String
where
    T: AsRef<[u8]>,
{
    byte_serialize(data.as_ref()).collect()
}

/// How often a subscription should pay
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NIP49BudgetPeriod {
    /// Resets daily at midnight
    Daily,
    /// Resets every week on sunday, midnight
    Weekly,
    /// Resets every month on the first, midnight
    Monthly,
    /// Resets every year on the January 1st, midnight
    Yearly,
}

impl fmt::Display for NIP49BudgetPeriod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NIP49BudgetPeriod::Daily => write!(f, "daily"),
            NIP49BudgetPeriod::Weekly => write!(f, "weekly"),
            NIP49BudgetPeriod::Monthly => write!(f, "monthly"),
            NIP49BudgetPeriod::Yearly => write!(f, "yearly"),
        }
    }
}

impl FromStr for NIP49BudgetPeriod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "day" => Ok(NIP49BudgetPeriod::Daily),
            "daily" => Ok(NIP49BudgetPeriod::Daily),
            "week" => Ok(NIP49BudgetPeriod::Weekly),
            "weekly" => Ok(NIP49BudgetPeriod::Weekly),
            "month" => Ok(NIP49BudgetPeriod::Monthly),
            "monthly" => Ok(NIP49BudgetPeriod::Monthly),
            "year" => Ok(NIP49BudgetPeriod::Yearly),
            "yearly" => Ok(NIP49BudgetPeriod::Yearly),
            _ => Err(()),
        }
    }
}

/// NIP49 URI Scheme
pub const NIP49_URI_SCHEME: &str = "nostr+walletauth";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NIP49Budget {
    pub time_period: NIP49BudgetPeriod,
    pub amount: u64,
}

impl fmt::Display for NIP49Budget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.amount, self.time_period)
    }
}

impl FromStr for NIP49Budget {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('/');
        let amount = split
            .next()
            .ok_or(Error::InvalidURI)?
            .parse()
            .map_err(|_| Error::InvalidURI)?;
        let time_period = split
            .next()
            .ok_or(Error::InvalidURI)?
            .parse()
            .map_err(|_| Error::InvalidURI)?;

        Ok(Self {
            time_period,
            amount,
        })
    }
}

/// Nostr Wallet Auth URI
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NIP49URI {
    /// App Pubkey
    pub public_key: PublicKey,
    /// URL of the relay of choice where the `App` is connected and the `Signer` must send and listen for messages.
    pub relay_url: Url,
    /// A random identifier that the wallet will use to identify the connection.
    pub secret: String,
    /// Required commands
    pub required_commands: Vec<Method>,
    /// Optional commands
    pub optional_commands: Vec<Method>,
    /// Budget
    pub budget: Option<NIP49Budget>,
    /// App's pubkey for identity verification
    pub identity: Option<PublicKey>,
}

impl FromStr for NIP49URI {
    type Err = Error;
    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(uri)?;

        if url.scheme() != NIP49_URI_SCHEME {
            return Err(Error::InvalidURIScheme);
        }

        if let Some(pubkey) = url.domain() {
            let public_key = PublicKey::from_str(pubkey)?;

            let mut relay_url: Option<Url> = None;
            let mut required_commands: Vec<Method> = vec![];
            let mut optional_commands: Vec<Method> = vec![];
            let mut budget: Option<NIP49Budget> = None;
            let mut secret: Option<String> = None;
            let mut identity: Option<PublicKey> = None;

            for (key, value) in url.query_pairs() {
                match key {
                    Cow::Borrowed("relay") => {
                        relay_url = Some(Url::parse(value.as_ref())?);
                    }
                    Cow::Borrowed("secret") => {
                        secret = Some(value.to_string());
                    }
                    Cow::Borrowed("required_commands") => {
                        required_commands = value
                            .split(' ')
                            .map(Method::from_str)
                            .collect::<Result<Vec<Method>, Error>>()?;
                    }
                    Cow::Borrowed("optional_commands") => {
                        optional_commands = value
                            .split(' ')
                            .map(Method::from_str)
                            .collect::<Result<Vec<Method>, Error>>()?;
                    }
                    Cow::Borrowed("budget") => {
                        budget = Some(NIP49Budget::from_str(value.as_ref())?);
                    }
                    Cow::Borrowed("identity") => {
                        identity = Some(PublicKey::from_str(value.as_ref())?);
                    }
                    _ => (),
                }
            }

            if required_commands.is_empty() {
                return Err(Error::InvalidURI);
            }

            if let Some((relay_url, secret)) = relay_url.zip(secret) {
                return Ok(Self {
                    public_key,
                    relay_url,
                    secret,
                    required_commands,
                    optional_commands,
                    budget,
                    identity,
                });
            }
        }

        Err(Error::InvalidURI)
    }
}

impl fmt::Display for NIP49URI {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{NIP49_URI_SCHEME}://{}?relay={}&secret={}&required_commands={}",
            self.public_key,
            url_encode(self.relay_url.to_string()),
            self.secret,
            url_encode(
                self.required_commands
                    .iter()
                    .map(|x| x.to_string())
                    .join(" ")
            ),
        )?;
        if !self.optional_commands.is_empty() {
            write!(
                f,
                "&optional_commands={}",
                url_encode(
                    self.optional_commands
                        .iter()
                        .map(|x| x.to_string())
                        .join(" ")
                )
            )?;
        }
        if let Some(budget) = &self.budget {
            write!(f, "&budget={}", url_encode(budget.to_string()))?;
        }
        if let Some(identity) = &self.identity {
            write!(f, "&identity={identity}")?;
        }
        Ok(())
    }
}
