# SUMMARY: Pre-fork Democli System

A client/server pre-fork system in Rust where a lightweight client transfers execution context to a long-running daemon. The daemon receives file descriptors and execution parameters, then runs the command with the provided resources.

## Epics

- **Research** (prefork-rs-gc3)
- **PFD-DAEMON** (prefork-rs-ufj): Server implementation for receiving execution contexts
- **PFC-CLIENT** (prefork-rs-ksn): Minimal client for transferring execution context

## Epic Purposes

- **Research**: Foundation work covering all technical prerequisites
- **PFD-DAEMON**: Long-running process accepting execution contexts via Unix sockets
- **PFC-CLIENT**: Lightweight client that discovers daemon and transfers execution context

## Research Areas

FD-TRANSFER, SOCKET-DISCOVERY, DAEMON-LIFECYCLE, EXECUTION-CONTEXT

## Key Technical Challenges

- **SOCKET-DISCOVERY**: Finding daemon socket across multiple strategies (local file, XDG runtime, XDG-project)
- **FD-TRANSFER**: Atomic transfer of file descriptors via Unix domain socket ancillary data
- **DAEMON-LIFECYCLE**: Double-fork daemonization, PID management, graceful shutdown

## Implementation Phases

1. Research & Exploration: Prototyping all technical components
2. PFD-DAEMON Foundation: Basic daemon with double-fork, PID files, socket server
3. PFC-CLIENT Implementation: Minimal client with discovery and context transfer
4. Enhancement: XDG-project mode, sd-notify, comprehensive error handling
