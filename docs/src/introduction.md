# ArkSync

ArkSync is an open-source, local-first environmental monitoring and control system developed by Station Knot. It is designed to monitor, regulate, and automate environments where operational continuity matters. ArkSync connects sensors and physical actuators to a local Hub that records measurements, presents them through dashboards such as gauges, charts, and KPIs, evaluates control conditions, triggers actions, raises alerts, and distributes configuration to edge devices.

The project is designed around three roles:

- A **Station** is a physical environment monitored and regulated by ArkSync. It can be as small as an aquarium or as large as an industrial facility.
- A **Hub** coordinates all or part of a Station and acts as its local control plane. It coordinates Knots, aggregates data, regulates the environment, and alerts operators. A smaller Station, such as a greenhouse, may use a single Hub. A larger Station, such as a factory, may use several Hubs to distribute monitoring and regulation across zones.
- A **Knot** runs close to the hardware, reads sensors, drives actuators, and exchanges messages with its Hub. If the Hub becomes unreachable, the Knot is designed to continue applying its last accepted configuration, preserving local monitoring and regulation until communication is restored.

ArkSync operates as a distributed system, allowing measurements and control to be organized across different environments and zones. **Fault tolerance** is a core design principle: a temporary network or Hub failure must not stop local monitoring and regulation. The long-term objective is to support laboratory-grade measurement workflows and industrial operational requirements with software and hardware designed for extreme conditions.

## Physical Infrastructure

The Hub is a Linux host application designed for x86_64 and AArch64 systems. It can run on any compatible Linux distribution, from desktop computers to Raspberry Pi devices.

Knots are designed to run on embedded platforms such as ESP32 boards. A Raspberry Pi Hub can also host a local Knot, allowing sensors and actuators to connect directly through interfaces such as I2C, UART, and GPIO. This provides a compact standalone deployment while preserving the same logical boundary used by remote Knots.

Station Knot can assist clients with infrastructure setup, support, and training. This book focuses on the free and open-source deployment for people who want to operate ArkSync themselves. The monitored environment is not prescribed: it may be a greenhouse, an aquarium, a home, or another environment with measurable and controllable conditions. More complex deployments can be designed with Station Knot.

Atlas Scientific sensors are currently the primary measurement hardware. Depending on the circuit and deployment, they can connect to a Knot through I2C or UART.

## Project state

ArkSync is under active development. This book documents the accepted system direction and the behavior that implementations are expected to follow; individual pages identify planned boundaries where the implementation is still evolving.

Continue with the [architecture overview](architecture/overview.md) for the main components and communication paths.
