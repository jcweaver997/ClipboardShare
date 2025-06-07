# ClipboardShare
Share your clipboard with all computers on the LAN

All computers on the LAN running this program will share a clipboard.

The implementation is now written in Rust (edition 2024) and sends clipboard
data over multicast using `bincode` serialization. Text and images are
supported. Large clipboard data is split into multiple UDP datagrams so it can
exceed the usual packet size limit.
