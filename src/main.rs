extern crate my_internet_ip;

fn main() {
	let ip: ::std::net::IpAddr = match my_internet_ip::get() {
		Ok(ip) => ip,
		Err(e) => panic!("Could not get IP: {:?}", e)
	};

	println!("{}", ip);
}
