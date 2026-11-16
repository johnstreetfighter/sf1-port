---
status: accepted
---

# Internet rooms go through a QUIC relay in its own crate

[Spec #123](https://github.com/johnstreetfighter/sf1-port/issues/123), approved on 2026-11-12, lets players on different home and carrier NATs play without port forwarding: both players dial out to a relay. For #124 the user chose QUIC (the spec's first option) through the `quinn` crate, and a new `sf-relay` crate. We chose QUIC over plain UDP with our own framing because it gives an authenticated, encrypted connection, certificate checks against the relay's hostname, and unreliable datagrams for rollback packets. The relay pins QUIC to its smallest packet size (1,200 bytes), so one 512-byte rollback packet plus its one-byte seat envelope always fits.

`sf-relay` holds three layers. The room code, room rules and wire format have no dependencies, so the engine can read a typed code without linking any networking. The QUIC service and the client sit behind the crate's `net` feature, which only `sf-host` enables. When the whole workspace is built, Cargo unifies features, so the engine's copy of `sf-relay` is built with `net` too, but the engine never calls it. The engine still names transport work only through `NetTransportCommand` and `NetTransportEvent`, which now include `HostInternetRoom` and `JoinInternetRoom`. The client runs its connection on its own worker thread, and `HostNetDriver` reads it through bounded queues without waiting. This extends the host transport described in [ADR-0018](0018-rollback-netplay-and-virtual-net-bus-on-the-1987-engine.md) section 4 and keeps the boundary of [ADR-0005](0005-game-binary-pure-screens-shared-host.md).

The relay never reads gameplay payloads. A room's seats are its connections: the host is seat 0 and guests take seats 1 to 15. A guest can only reach the host, and each seat is the route number the engine already uses. Codes are six symbols from a 31-symbol alphabet with no 0, O, 1, I or L, so they cannot be confused with LAN address codes. Rooms close when the host leaves, after 15 idle minutes, or after 12 hours. Creates and joins are throttled per IPv4 address or IPv6 /64, and failed joins count against the limit. Each connection also has a traffic budget.

Choosing the default relay and running it (#130), the community relay setting and its menu (#128), and spectating and Dramatic Battle over the relay (#127) are separate tickets.
