# Drop Sharing

Share files with nearby devices over Wi-Fi Direct. Transfers require the
receiver's approval and use encrypted file chunks.

## Build and run

```sh
cargo build --release
cargo run --release
```

The app opens a desktop window where you can send files or receive them.
Received files go to your Downloads directory by default; you can change this
in Settings.

Command line use is also available:

```sh
cargo run --release -- start-receiver
cargo run --release -- start-sender <DIRECTORY>
```

Use `start-receiver --daemon` to keep receiving and approve transfers through
desktop notifications.

The app needs access to the `wpa_supplicant` control socket and an IP-configured
Wi-Fi Direct group. It uses the only wireless interface found under
`/sys/class/net` by default. Set `DROP_SHARING_WPA_CTRL_PATH` to the socket path
(for example, `/run/wpa_supplicant/wlp8s0`) if you have multiple wireless
interfaces or use another socket location.
