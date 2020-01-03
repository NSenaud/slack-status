#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::str::FromStr;

    use slack_status::*;

    #[test]
    fn test_status_from_location_0() {
        let config = Config {
            token: String::from_str("xxx").unwrap(),
            defaults: None,
            ip_request_address: None,
            locations: Vec::new(),
        };
        let client = SlackStatus::from(&config).unwrap();
        let status = client.status_from_location(&IpAddr::from_str("123.45.67.89").unwrap());

        assert!(status.is_none());
    }

    #[test]
    fn test_status_from_location_1() {
        let config = Config {
            token: String::from_str("xxx").unwrap(),
            defaults: None,
            ip_request_address: None,
            locations: vec![
                Location {
                    ip: IpAddr::from_str("123.45.67.89").unwrap(),
                    text: String::from_str("here!").unwrap(),
                    emoji: String::from_str(":yolo:").unwrap(),
                    expire_after_hours: Some(1),
                },
            ],
        };
        let client = SlackStatus::from(&config).unwrap();
        let status = client.status_from_location(&IpAddr::from_str("123.45.67.89").unwrap());

        assert_eq!(status.unwrap().text, "here!");
    }

    #[test]
    fn test_status_from_location_2() {
        let config = Config {
            token: String::from_str("xxx").unwrap(),
            defaults: None,
            ip_request_address: None,
            locations: vec![
                Location {
                    ip: IpAddr::from_str("123.45.67.89").unwrap(),
                    text: String::from_str("here!").unwrap(),
                    emoji: String::from_str(":yolo:").unwrap(),
                    expire_after_hours: Some(1),
                },
                Location {
                    ip: IpAddr::from_str("98.76.54.32").unwrap(),
                    text: String::from_str("there!").unwrap(),
                    emoji: String::from_str(":yolo:").unwrap(),
                    expire_after_hours: Some(1),
                },
            ],
        };
        let client = SlackStatus::from(&config).unwrap();

        let status = client.status_from_location(&IpAddr::from_str("123.45.67.89").unwrap());

        assert_eq!(status.unwrap().text, "here!");
    }
}
