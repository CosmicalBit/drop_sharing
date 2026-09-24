use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use tokio::time::{Instant, sleep, timeout};
use wpactrl::ClientAttached;

use crate::{
    discovery::connection::{Connection},
    encryption::cipher::TransferError,
    wpa::parser::{P2pGroupStarted, P2pRole, parse_group_started},
};

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const GROUP_TIMEOUT: Duration = Duration::from_secs(30);
const TCP_TIMEOUT: Duration = Duration::from_secs(30);

struct Wpa {
    wpa: ClientAttached,
}

pub struct P2pConnection {
    pub tcp: Connection,
    _group: P2pGroupGuard,
}

struct P2pGroupGuard {
    wpa: ClientAttached,
    interface: String,
}

impl Drop for P2pGroupGuard {
    fn drop(&mut self) {
        let _ = self.wpa.request(&format!("P2P_GROUP_REMOVE {}", self.interface));
    }
}

impl Wpa {
    fn init() -> Result<Self, TransferError> {
        let wpa = wpactrl::Client::builder()
            .ctrl_path(control_path()?)
            .open()?
            .attach()?;

        Ok(Self { wpa })
    }

    fn request_ok(&mut self, command: &str) -> Result<(), TransferError> {
        let response = self.wpa.request(command)?;
        if response.trim() == "OK" {
            Ok(())
        } else {
            Err(io::Error::other(format!("{command}: {}", response.trim())).into())
        }
    }

    async fn search(&mut self) -> Result<String, TransferError> {
        self.request_ok("P2P_FIND")?;

        let mut receivers = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Some(event) = self.wpa.recv()? {
                if event_name(&event) == Some("P2P-DEVICE-FOUND") {
                    if let Some(address) = get_p2p_device_address(&event) {
                        if !receivers.iter().any(|receiver| receiver == address) {
                            receivers.push(address.to_owned());
                        }
                    }
                }
            } else {
                sleep(POLL_INTERVAL).await;
            }
        }
        self.request_ok("P2P_STOP_FIND")?;

        choose_receiver(&receivers)
    }

    async fn wait_for_group(&mut self) -> Result<P2pGroupStarted, TransferError> {
        let deadline = Instant::now() + GROUP_TIMEOUT;
        while Instant::now() < deadline {
            if let Some(event) = self.wpa.recv()? {
                if let Some(group) = parse_group_started(&event) {
                    return Ok(group);
                }
                if matches!(event_name(&event), Some("P2P-GO-NEG-FAILURE" | "P2P-GROUP-FORMATION-FAILURE")) {
                    return Err(io::Error::other(format!("Wi-Fi Direct group formation failed: {event}")).into());
                }
            } else {
                sleep(POLL_INTERVAL).await;
            }
        }
        Err(io::Error::new(io::ErrorKind::TimedOut, "Wi-Fi Direct group formation timed out").into())
    }

    async fn connect_tcp(mut self) -> Result<P2pConnection, TransferError> {
        let group = self.wait_for_group().await?;
        let guard = P2pGroupGuard { wpa: self.wpa, interface: group.interface };
        let tcp = match group.role {
            P2pRole::GroupOwner => timeout(TCP_TIMEOUT, Connection::new_listen())
                .await
                .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Wi-Fi Direct TCP accept timed out"))?
                .map_err(TransferError::from)?,
            P2pRole::Client => {
                let go_ip = group.go_ip.ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "Wi-Fi Direct group has no owner IP address")
                })?;
                let deadline = Instant::now() + TCP_TIMEOUT;
                loop {
                    match timeout(deadline.saturating_duration_since(Instant::now()), Connection::connect(go_ip)).await {
                        Ok(Ok(connection)) => break connection,
                        Ok(Err(_)) if Instant::now() < deadline => {
                            sleep(POLL_INTERVAL).await;
                        }
                        Ok(Err(error)) => return Err(error.into()),
                        Err(_) => return Err(io::Error::new(io::ErrorKind::TimedOut, "Wi-Fi Direct TCP connect timed out").into()),
                    }
                }
            }
        };
        Ok(P2pConnection { tcp, _group: guard })
    }

    async fn connect(mut self, address: &str) -> Result<P2pConnection, TransferError> {
        self.request_ok(&format!("P2P_CONNECT {address} pbc"))?;
        self.connect_tcp().await
    }

    async fn accept(mut self) -> Result<P2pConnection, TransferError> {
        self.request_ok("P2P_FIND")?;
        loop {
            if let Some(event) = self.wpa.recv()? {
                if matches!(event_name(&event), Some("P2P-GO-NEG-REQUEST" | "P2P-PROV-DISC-PBC-REQ")) {
                    let address = event.split_whitespace().nth(1).ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "Wi-Fi Direct request has no peer address")
                    })?;
                    self.request_ok(&format!("P2P_CONNECT {address} pbc"))?;
                    return self.connect_tcp().await;
                }
            } else {
                sleep(POLL_INTERVAL).await;
            }
        }
    }
}

fn control_path() -> io::Result<PathBuf> {
    if let Some(path) = std::env::var_os("DROP_SHARING_WPA_CTRL_PATH") {
        if path.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "DROP_SHARING_WPA_CTRL_PATH is empty"));
        }
        return Ok(PathBuf::from(path));
    }

    let mut interface = None;
    for entry in fs::read_dir("/sys/class/net")? {
        let entry = entry?;
        if entry.path().join("wireless").is_dir() {
            if interface.replace(entry.file_name()).is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "multiple Wi-Fi interfaces found; set DROP_SHARING_WPA_CTRL_PATH",
                ));
            }
        }
    }
    let interface = interface.ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "no Wi-Fi interface found; set DROP_SHARING_WPA_CTRL_PATH")
    })?;
    Ok(Path::new("/run/wpa_supplicant").join(interface))
}

pub fn choose_receiver(receivers: &[String]) -> Result<String, TransferError> {
    if receivers.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "no receivers found").into());
    }

    println!("Available receivers:");
    for (index, receiver) in receivers.iter().enumerate() {
        println!("{}. {receiver}", index + 1);
    }

    loop {
        print!("Choose a receiver (1-{}): ", receivers.len());
        io::stdout().flush()?;

        let mut input = String::new();
        if io::stdin().read_line(&mut input)? == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "no receiver selected").into());
        }

        if let Ok(index) = input.trim().parse::<usize>() {
            if let Some(receiver) = index.checked_sub(1).and_then(|index| receivers.get(index)) {
                return Ok(receiver.clone());
            }
        }

        println!("Enter a number from 1 to {}.", receivers.len());
    }
}

fn event_name(event: &str) -> Option<&str> {
    let event = event.trim_start();
    let event = if let Some(rest) = event.strip_prefix('<') {
        rest.split_once('>').map(|(_, event)| event).unwrap_or(event)
    } else {
        event
    };
    event.split_whitespace().next()
}

fn get_p2p_device_address(event: &str) -> Option<&str> {
    event.split_whitespace().find_map(|field| field.strip_prefix("p2p_dev_addr="))
}

pub async fn sender_connect() -> Result<P2pConnection, TransferError> {
    let mut wpa = Wpa::init()?;
    let address = wpa.search().await?;
    wpa.connect(&address).await
}

pub async fn receiver_connect() -> Result<P2pConnection, TransferError> {
    Wpa::init()?.accept().await
}
