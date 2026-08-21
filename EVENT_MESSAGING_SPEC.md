# ArkSync Event Messaging V1

## Purpose

ArkSync needs one readable messaging path between a Hub and a Knot, whether the
Knot runs inside the Hub process or on remote hardware. The messaging contract
must remain independent from Tokio, Embassy, MQTT, LoRa, Zenoh, and database
concerns.

This refactor separates five responsibilities:

1. synchronous in-process event routing;
2. the shared Hub/Knot wire contract;
3. the portable Knot state machine;
4. runtime-specific scheduling and links;
5. concrete local or remote transports.

The V1 delivery model is at-least-once. It is not event sourcing: the Hub keeps
only processing receipts for idempotence, while PostgreSQL remains the durable
source of truth for station state.

## Crate Boundaries

`arksync-bus` owns generic envelopes, identifiers, Postcard helpers, and a
synchronous `EventRouter`. It has no knowledge of MQTT topics, persistence, the
Hub, or the Knot protocol.

`arksync-knot-protocol` owns the bilateral Hub/Knot contract. It is `no_std`
with `alloc`, owns its wire DTOs, and is shared by the Hub, Knot, and protocol
tests. Internal sensor and actuator events are translated explicitly at the
boundary instead of becoming the wire ABI.

`arksync-knot` owns the portable Knot state machine and runtime adapters. The
state machine is synchronous; Tokio or Embassy drives it and supplies clocks,
event identifiers, hardware adapters, and a message link.

`arksync-hub` owns asynchronous application handlers, transaction boundaries,
the inbox of processed messages, and orchestration of local or remote Knot
links.

## Local Event Routing

`EventRouter` is an in-memory, synchronous publish/subscribe router.

- handlers receive a borrowed envelope;
- publishing continues after a handler rejection;
- publishing returns matched, delivered, and rejected counts;
- `Delivery` and `Persistence` do not belong to the router;
- `EventPublisher` is only a borrowed local publishing handle;
- channels are used only at real task/runtime boundaries.

## Protocol Frame

A wire frame contains a fixed ArkSync magic prefix, a protocol version, and a
Postcard-encoded `KnotEnvelope`. Message transports such as MQTT, LoRa, and
Zenoh carry this complete frame. Stream transports such as TCP may add their
own length delimiter outside the ArkSync frame.

`KnotEnvelope` is an `EventEnvelope<KnotMessage, KnotMessageSource>`.
`KnotMessageSource` identifies either a Hub by its logical ID or a Knot by its
stable hardware UID. A remote transport may authenticate the peer separately;
the claimed source remains useful for routing, auditing, and local links.

Protocol enums are append-only within a protocol version. Incompatible wire
changes require a version increment and updated golden-byte tests.

## V1 Messages

The root `KnotMessage` groups control and sensor messages.

Control messages cover:

- `Hello`, including capabilities and the last applied configuration version;
- `Ack::Hello`, correlating the Hello EventId and carrying the current config;
- `Ack::Processed`, correlating any other successfully processed message;
- `Nack`, correlating a rejected message with a typed reason;
- `ConfigApplied` and `ConfigRejected`, confirming whether a Knot applied the
  configuration contained in its Hello ACK.

Sensor messages cover the current plugged, provisioned, provisioning-conflict,
and measurement flows. Measurements use a batch payload from V1; the current
UART adapter initially emits singleton batches.

The Hub-provided `KnotConfig` has a global version, the logical Knot ID, sensor
bindings, and actuator configurations needed by the current runtime.

## Delivery Semantics

Hello, sensor messages, `ConfigApplied`, and `ConfigRejected` require an ACK.
ACK and NACK messages do not themselves require an ACK.

A retry of the same logical message preserves its EventId. A corrected or new
message receives a new EventId. The Knot stores unacknowledged messages in a
bounded in-memory outbox with configurable capacity and backoff. V1 defaults
to 256 entries, a one-second initial delay, a 30-second maximum delay, and no
attempt limit. A full outbox returns an explicit error and increments an
observable overflow counter.

NACK reasons determine whether the same message is retried. Transport and Hub
infrastructure failures produce no ACK, causing the normal timeout path. A
terminal semantic rejection removes the pending message; corrected data must
be emitted with a new EventId.

The Hub stores a processing receipt in the same transaction as the business
effect. A duplicate EventId is acknowledged without replaying that effect. A
failed transaction stores no receipt and sends no ACK.

## Runtime And Transport Model

The portable Knot runtime exposes synchronous operations for startup, inbound
messages, local sensor observations, and retry ticks. Runtime adapters decide
when to call those operations and how to deliver their outputs.

`MessageLink<M>` is a point-to-point asynchronous port with `send` and
`receive`. It does not select a runtime. A local Tokio implementation hides its
two bounded channels behind a pair of duplex endpoints. A future Embassy,
MQTT, LoRa, or Zenoh implementation will provide the same semantics.

The Hub owns one endpoint for each connected Knot. Application handlers return
protocol responses instead of receiving Tokio senders. One malformed or
temporarily failing message must not terminate the Hub runtime loop.

## Initial Scope

This branch migrates the EventRouter, Hello/config flow, and sensor messages.
The current actuator runtime messaging remains functional but is isolated as a
legacy local path until its own protocol migration.

The V1 outbox is not persisted across an ESP32 reboot. Flash-backed outbox
storage, MQTT, LoRa, Zenoh, and multi-peer remote gateways are future adapters
or extensions, not dependencies of this refactor.

## Acceptance Tests

- synchronous no-runtime routing, filtering, fan-out, and rejection reporting;
- Postcard round trips, golden frames, invalid magic/version, and short buffers;
- Hello registration/config and explicit config-applied/config-rejected flows;
- outbox ACK, retry, backoff, NACK, EventId preservation, and overflow;
- duplicate Hello and measurement batches with one durable business effect;
- local Hub/Knot exchange through a Tokio link;
- remote exchange and reconnect through a TCP loopback Postcard adapter;
- `no_std` checks for the bus, protocol, and portable Knot runtime;
- unchanged Raspberry Pi sensor and relay behavior.

## Documentation Exit

Before merging, stable architectural decisions move into the mdBook and public
Rustdocs. This temporary file and the exploratory TODO in the Tokio runtime are
then removed.
