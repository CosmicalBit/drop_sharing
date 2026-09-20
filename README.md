# drop_sharing

A airdrop like tool 

Devices discover each other using UDP broadcast and establish a TCP
connection for the actual transfer, the main advantage is that you can connect to any device on the same network, you dont need to be physical close too the device.
The connection performs an authenticated key exchange before any files are transferred.

File contents are transferred in chunks and encrypted using
ChaCha20-Poly1305.

## How it works

1. Discover devices over UDP.
2. Establish a TCP connection.
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

To keep the receiver running and confirm incoming transfers through desktop
notifications, use daemon mode:

```sh
cargo run --release -- start-receiver --daemon
```

On the sending device, provide the path to the directory to send:

```sh
cargo run --release -- start-sender <DIRECTORY>
```
