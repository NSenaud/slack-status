#[macro_use]
extern crate serde_json;
extern crate my_internet_ip;
extern crate reqwest;

fn main() {
	let ip: ::std::net::IpAddr = match my_internet_ip::get() {
		Ok(ip) => ip,
		Err(e) => panic!("Could not get IP: {:?}", e)
	};

	println!("{}", ip);

    let client = reqwest::Client::new();
    let res: reqwest::Response = match client.post("https://slack.com/api/users.profile.set")
        .bearer_auth("token")
        .json(
            &json!({
                "profile": {
                    "status_text": "test",
                    "status_emoji": ":mountain_railway:",
                    "status_expiration": 0
                }
            }))
        .send() {
            Ok(res) => res,
            Err(e) => panic!("Failed to change status: {:?}", e),
    };

    println!("{:#?}", res);
}
