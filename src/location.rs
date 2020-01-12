use std::fmt;
use std::net::IpAddr;

/// A Location matches an IP address (either IPv4 or IPv6) with a Status.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Location {
    pub ip: IpAddr,
    pub text: String,
    pub emoji: String,
    pub expire_after_hours: Option<i64>,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} => {} {}", self.ip, self.emoji, self.text)
    }
}
