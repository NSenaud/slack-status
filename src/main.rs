#[macro_use]
extern crate serde_json;
extern crate my_internet_ip;
extern crate reqwest;

use std::net::{IpAddr, Ipv4Addr};

fn main() {
    let _home = IpAddr::V4(Ipv4Addr::new(176, 187, 98, 85));
    let _saint_martin = IpAddr::V4(Ipv4Addr::new(78, 193, 77, 48));

    let ip: ::std::net::IpAddr = match my_internet_ip::get() {
        Ok(ip) => ip,
        Err(e) => panic!("Could not get IP: {:?}", e)
    };

    let status_text: &str;
    let status_emoji: &str;

    if ip == _saint_martin {
        println!("Saint-Martin");
        status_text = "ipso Saint-Martin";
        status_emoji = ":ipso:";
    } else if ip == _home {
        println!("Maison");
        status_text = "maison";
        status_emoji = ":house:";
    } else {
        println!("En deplacement");
        status_text = "en deplacement";
        status_emoji = ":mountain_railway:";
    }

    println!("{}", ip);

    let client = reqwest::Client::new();
    let res: reqwest::Response = match client.post("https://slack.com/api/users.profile.set")
        .bearer_auth("")
        .json(
            &json!({
                "profile": {
                    "status_text": status_text,
                    "status_emoji": status_emoji,
                    "status_expiration": 0
                }
            }))
        .send() {
            Ok(res) => res,
            Err(e) => panic!("Failed to change status: {:?}", e),
    };

    println!("{:#?}", res);
}
