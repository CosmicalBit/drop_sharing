# drop_sharing

A file sharing tool using Wi-Fi Direct.

Devices discover each other and form a Wi-Fi Direct group through wpa_supplicant.
The file protocol uses a TCP connection over that group and performs an
authenticated key exchange before transferring files.

File contents are transferred in chunks and encrypted using
ChaCha20-Poly1305.

## How it works

1. Discover a nearby device and form a Wi-Fi Direct group.
2. Establish a TCP connection over the group.
3. Exchange and verify identities.
4. Establish a shared secret.
5. Confirm the transfer.
6. Transfer authenticated, encrypted file chunks.

## Status

The basic transfer protocol is working. The project is still experimental
and hasn't been independently security audited.

## Building

```sh
cargo build --release
```

## How to use

On the receiving device, start the receiver:

```sh
cargo run --release -- start-receiver
```

The program uses the only wireless interface found under `/sys/class/net`.
If there are multiple interfaces or the wpa_supplicant control socket is in
another location, set `DROP_SHARING_WPA_CTRL_PATH` to its socket path, for
example `/run/wpa_supplicant/wlp8s0`. The user running the program needs
access to that socket, and the Wi-Fi Direct group needs IP configuration.

To keep the receiver running and confirm incoming transfers through desktop
notifications, use daemon mode:

```sh
cargo run --release -- start-receiver --daemon
```

On the sending device, provide the path to the directory to send:

```sh
cargo run --release -- start-sender <DIRECTORY>
```
