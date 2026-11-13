# Architecture Overview

ArkSync separates station coordination from direct hardware access. The Hub owns persistent station state and user-facing workflows, while Knots execute close to sensors and actuators.

The following diagram shows the intended high-level topology. The local Knot path is used by the current Raspberry Pi deployment. Remote Knots exchange the same protocol messages through transport-specific adapters. The protocol is designed independently of the transport technology, leaving concrete communication choices open.

```mermaid
flowchart LR
    subgraph RemoteEdge["Remote edge"]
        RemoteSensors["Atlas Scientific sensors<br/>I2C / UART"]
        EspKnot["Remote Knot runtime<br/>ESP32"]
        RemoteSensors --> EspKnot
    end

    subgraph HubAppliance["Hub appliance - Raspberry Pi"]
        LocalSensors["Atlas Scientific sensors<br/>I2C / UART"]
        LocalKnot["Local Knot runtime"]
        Hub["Hub runtime"]
        Dashboard["Dashboard"]
        Postgres[("PostgreSQL")]

        LocalSensors --> LocalKnot
        LocalKnot <-->|"Knot protocol<br/>Tokio message link"| Hub
        Hub <--> Postgres
        Dashboard <--> Hub
    end

    EspKnot -.->|"Knot protocol<br/>Transport adapter"| Hub
```

## Knot

The Knot owns portable protocol messages and hardware-facing execution. Its reusable domain and runtime contracts are `no_std` first. Platform adapters provide concrete scheduling, transport, sensor, and actuator integrations:

- the Raspberry Pi Hub uses a Tokio-backed local Knot runtime;
- a remote ESP32 Knot will use an embedded runtime and a transport adapter suited to its deployment;
- both paths preserve the same protocol-level messages.

## Communication and transport

ArkSync defines its own Hub/Knot communication protocol. Message semantics and the portable protocol core remain independent of the underlying link. Transport adapters may use LoRaWAN, Wi-Fi, or other communication technologies according to deployment needs.

LoRaWAN is currently the preferred candidate for communication across long buildings and outdoor deployments.

See [event messaging](event-messaging.md#runtime-and-communication-boundaries) for the EventRouter, protocol, and transport boundaries between Knot and Hub runtimes.

## Hub

The Hub is the station coordination boundary. It receives Knot messages, applies application workflows, persists station state, and supplies the dashboard. The Hub is a host runtime and may use Tokio, SQLx, and operating-system services.

On a standalone Raspberry Pi deployment, the Hub also launches a local Knot. This gives directly connected I2C, UART, and GPIO hardware the same logical boundary as hardware attached to a future remote Knot.

## PostgreSQL

PostgreSQL is the Hub's persistent source of truth. It stores station identities, registered hardware, measurements, dashboard configuration, actuator configuration, and other durable state. Runtime caches and event streams may improve responsiveness, but they do not replace persisted state.

## Dashboard

The dashboard is the user-facing projection of Hub state. It obtains persisted measurements and configuration through Hub application workflows and can later consume realtime updates without becoming responsible for hardware communication or domain rules.

Continue with [event messaging](event-messaging.md) for the protocol, delivery semantics, and runtime boundaries used between Hubs and Knots.
