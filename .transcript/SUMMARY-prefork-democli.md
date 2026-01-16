# SUMMARY: Pre-fork Democli System

A client/server pre-fork system in Rust where a lightweight client transfers execution context to a long-running daemon. The daemon receives file descriptors and execution parameters, then runs the command with the provided resources.

## Epics

| Short-Key | Purpose |
|-----------|---------|
| Research (gc3) | Foundation work covering all technical prerequisites for the prefork system. This epic contains research tickets investigating file descriptor transfer mechanisms, socket discovery patterns, daemon lifecycle management, and execution context serialization. |
| PFD-DAEMON (ufj) | Long-running process accepting execution contexts via Unix sockets. Implements double-fork daemonization, PID file management, Unix domain socket server, and command execution with transferred descriptors. Initial command: `add` to sum arguments. |
| PFC-CLIENT (ksn) | Lightweight client that discovers daemon and transfers execution context. Philosophy: keep incredibly small and simple. Finds socket quickly, transfers context + file descriptors, auto-launches daemon if needed, then terminates. |

## Research Areas & Challenges

| Short-Key | Focus |
|-----------|-------|
| FD-TRANSFER | Atomic transfer of file descriptors via Unix domain socket ancillary data using the `sendfd` crate. Must understand descriptor lifecycle, multi-descriptor patterns, and handle transfer failures gracefully. Critical for moving stdin/stdout/stderr from client to daemon. |
| SOCKET-DISCOVERY | Finding daemon socket across multiple strategies: local file mode (`./pfd.sock`), XDG runtime mode (`$XDG_RUNTIME_DIR/pfd.sock`), and XDG-project mode with project name middle-fix. Requires fallback strategies, atomic socket creation, and proper permission handling. |
| DAEMON-LIFECYCLE | Double-fork daemonization for background processes with PID file management in XDG locations. Needs clean shutdown, process group management, signal handling, and health checking. Must prevent multiple instances and handle active executions during termination. |
| EXECUTION-CONTEXT | Serialization and transfer of command, arguments, working directory, and descriptors. Must design efficient protocol, validate context security, handle working directory changes, and ensure proper argument array encoding before execution. |

## Implementation Phases

1. Research & Exploration: Prototyping all technical components
2. PFD-DAEMON Foundation: Basic daemon with double-fork, PID files, socket server
3. PFC-CLIENT Implementation: Minimal client with discovery and context transfer
4. Enhancement: XDG-project mode, sd-notify, comprehensive error handling
