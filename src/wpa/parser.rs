use std::net::Ipv4Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum P2pRole {
    GroupOwner,
    Client,
}

pub struct P2pGroupStarted {
    pub interface: String,
    pub role: P2pRole,
    pub go_ip: Option<Ipv4Addr>,
}

pub fn parse_group_started(event: &str) -> Option<P2pGroupStarted> {
    let event = if let Some(rest) = event.strip_prefix('<') {
        rest.split_once('>')?.1
    } else {
        event
    };
    let mut fields = event.split_whitespace();
    if fields.next()? != "P2P-GROUP-STARTED" {
        return None;
    }

    let interface = fields.next()?.to_owned();
    let role = match fields.next()? {
        "GO" => P2pRole::GroupOwner,
        "client" => P2pRole::Client,
        _ => return None,
    };
    let mut quoted = false;
    let mut go_ip = None;
    for field in fields {
        if !quoted {
            go_ip = go_ip.or_else(|| field.strip_prefix("go_ip_addr=").and_then(|address| address.parse().ok()));
        }
        if field.bytes().filter(|byte| *byte == b'"').count() % 2 != 0 {
            quoted = !quoted;
        }
    }

    Some(P2pGroupStarted { interface, role, go_ip })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_client_group_address_from_wpa_event() {
        let event = "<3>P2P-GROUP-STARTED p2p-wlan0-0 client ssid=\"DIRECT-test\" ip_addr=192.168.49.2 go_ip_addr=192.168.49.1";
        let group = parse_group_started(event).unwrap();

        assert_eq!(group.role, P2pRole::Client);
        assert_eq!(group.go_ip, Some(Ipv4Addr::new(192, 168, 49, 1)));
    }

    #[test]
    fn parses_group_owner_event_without_ip_fields() {
        let event = "<3>P2P-GROUP-STARTED p2p-wlan0-0 GO ssid=\"DIRECT-test\" freq=2412";
        let group = parse_group_started(event).unwrap();

        assert_eq!(group.role, P2pRole::GroupOwner);
        assert_eq!(group.interface, "p2p-wlan0-0");
    }

    #[test]
    fn ignores_ip_looking_text_inside_ssid() {
        let event = "P2P-GROUP-STARTED p2p-wlan0-0 client ssid=\"DIRECT go_ip_addr=10.0.0.1\" go_ip_addr=192.168.49.1";
        let group = parse_group_started(event).unwrap();

        assert_eq!(group.go_ip, Some(Ipv4Addr::new(192, 168, 49, 1)));
    }
}
