#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::str::FromStr;

    use slack_status::*;

    #[test]
    fn test_status_from_location_0() {
        let client = SlackStatus::from(Config {
            token: String::from_str("xxx").unwrap(),
            defaults: None,
            ip_request_address: None,
            locations: Vec::new(),
        }).unwrap();

        let status = client.status_from_location(&IpAddr::from_str("123.45.67.89").unwrap());

        assert!(status.is_none());
    }

    #[test]
    fn test_status_from_location_1() {
        let client = SlackStatus::from(Config {
            token: String::from_str("xxx").unwrap(),
            defaults: None,
            ip_request_address: None,
            locations: vec![
                Location {
                    ip_addresses: vec![IpAddr::from_str("123.45.67.89").unwrap()],
                    text: String::from_str("here!").unwrap(),
                    emoji: String::from_str(":yolo:").unwrap(),
                },
            ],
        }).unwrap();

        let status = client.status_from_location(&IpAddr::from_str("123.45.67.89").unwrap());

        assert_eq!(status.unwrap().text, "here!");
    }

    #[test]
    fn test_status_from_location_2() {
        let client = SlackStatus::from(Config {
            token: String::from_str("xxx").unwrap(),
            defaults: None,
            ip_request_address: None,
            locations: vec![
                Location {
                    ip_addresses: vec![IpAddr::from_str("123.45.67.89").unwrap()],
                    text: String::from_str("here!").unwrap(),
                    emoji: String::from_str(":yolo:").unwrap(),
                },
                Location {
                    ip_addresses: vec![IpAddr::from_str("98.76.54.32").unwrap()],
                    text: String::from_str("there!").unwrap(),
                    emoji: String::from_str(":yolo:").unwrap(),
                },
            ],
        }).unwrap();

        let status = client.status_from_location(&IpAddr::from_str("123.45.67.89").unwrap());

        assert_eq!(status.unwrap().text, "here!");
    }

    #[test]
    fn test_status_from_location_3() {
        let client = SlackStatus::from(Config {
            token: String::from_str("xxx").unwrap(),
            defaults: None,
            ip_request_address: None,
            locations: vec![
                Location {
                    ip_addresses: vec![
                        IpAddr::from_str("87.65.43.21").unwrap(),
                        IpAddr::from_str("123.45.67.89").unwrap(),
                    ],
                    text: String::from_str("here!").unwrap(),
                    emoji: String::from_str(":yolo:").unwrap(),
                },
                Location {
                    ip_addresses: vec![IpAddr::from_str("98.76.54.32").unwrap()],
                    text: String::from_str("there!").unwrap(),
                    emoji: String::from_str(":yolo:").unwrap(),
                },
            ],
        }).unwrap();

        let status = client.status_from_location(&IpAddr::from_str("123.45.67.89").unwrap());

        assert_eq!(status.unwrap().text, "here!");
    }

    #[test]
    fn test_status_from_location_4() {
        let client = SlackStatus::from(Config {
            token: String::from_str("xxx").unwrap(),
            defaults: None,
            ip_request_address: None,
            locations: vec![
                Location {
                    ip_addresses: vec![
                        IpAddr::from_str("87.65.43.21").unwrap(),
                        IpAddr::from_str("123.45.67.89").unwrap(),
                    ],
                    text: String::from_str("here!").unwrap(),
                    emoji: String::from_str(":yolo:").unwrap(),
                },
                Location {
                    ip_addresses: vec![
                        IpAddr::from_str("98.76.54.32").unwrap(),
                        IpAddr::from_str("123.45.67.89").unwrap(),
                    ],
                    text: String::from_str("there!").unwrap(),
                    emoji: String::from_str(":yolo:").unwrap(),
                },
            ],
        }).unwrap();

        let status = client.status_from_location(&IpAddr::from_str("123.45.67.89").unwrap());

        assert!(status.is_none());
    }
}
