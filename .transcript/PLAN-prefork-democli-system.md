# PLAN: Pre-fork Democli System

## Problem Statement

Build a client/server pre-fork system in Rust where a lightweight client transfers execution context to a long-running daemon. The daemon receives file descriptors and execution parameters, then runs the command with the provided resources.

## System Overview

### Components

**PFD (Daemon)**
- Long-running process that accepts execution contexts
- Receives file descriptors (stdin, stdout, stderr) via Unix domain socket
- Executes commands in the provided working directory with given arguments
- Manages multiple concurrent execution contexts
- Supports daemonization via double-fork technique

**PFC (Client)**
- Minimal client that discovers and connects to daemon
- Transfers execution context (command, args, cwd, descriptors) via socket
- Terminates immediately after successful transfer
- Supports auto-launching daemon if not found

### Execution Context Structure

| Component | Description |
|-----------|-------------|
| command | Program name to execute |
| args | Array of command-line arguments |
| cwd | Current working directory for execution |
| descriptors | File descriptors array [stdin, stdout, stderr] |

## Architecture Views

### Component Architecture

```
┌─────────────┐                    ┌─────────────┐
│     PFC     │                    │     PFD     │
│  (Client)   │                    │  (Daemon)   │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │  1. Socket Discovery             │
       ├──────────────────────────────────>│
       │                                  │
       │  2. Transfer Context + FDs       │
       ├──────────────────────────────────>│
       │                                  │
       │  3. Execute                      │
       │                                  ├─────────┐
       │                                  │         │
       │  4. Client Exits                 │         ▼
       │                                  │   Command
       │                                  │ Execution
       │                                  └─────────┘
```

### Data Flow

1. **Discovery Phase**
   - PFC searches for socket using configured strategy
   - Falls back through modes if socket not found
   - Optionally launches PFD with --create flag

2. **Transfer Phase**
   - PFC serializes execution context
   - Opens Unix domain socket
   - Sends context and ancillary data (file descriptors)
   - Waits for acknowledgment

3. **Execution Phase**
   - PFD receives context and descriptors
   - Validates execution parameters
   - Changes to working directory
   - Spawns child process with transferred descriptors
   - Returns execution status to client (optional)

## Research Areas

### FD-TRANSFER: File Descriptor Transfer

**Investigation needed:**
- `sendfd` crate API surface and capabilities
- Unix domain socket ancillary data structure
- Multi-descriptor transfer patterns
- Descriptor lifecycle and ownership transfer
- Error handling for failed transfers

**Key questions:**
- How to ensure atomic transfer of multiple descriptors?
- What happens to descriptors if transfer fails mid-stream?
- How to handle descriptor closure on both sides?

### SOCKET-DISCOVERY: Socket Discovery Strategies

**Discovery modes:**
1. Local file mode: `./pfd.sock` or `.pfd.sock`
2. XDG runtime mode: `$XDG_RUNTIME_DIR/pfd.sock`
3. XDG-project mode: `$XDG_RUNTIME_DIR/pfd.<project-name>.sock`

**Research needed:**
- Rust `directories` crate integration
- Fallback strategy ordering and priority
- Socket file permissions and atomic creation
- Project name extraction and validation
- Cross-platform XDG path handling

### DAEMON-LIFECYCLE: Process Management

**Research areas:**
- `fork` crate double-fork implementation
- PID file creation and locking in XDG locations
- Daemon startup validation and health checks
- Signal handling for graceful shutdown
- Process group management
- Auto-launch daemon functionality (--create/-C flag)

**Key concerns:**
- Preventing multiple daemon instances
- Clean shutdown of active executions
- PID file cleanup on exit

### EXECUTION-CONTEXT: Context Serialization

**Research needed:**
- Serialization format for socket transfer (bincode, serde, etc.)
- Working directory handling and validation
- Argument array encoding
- Security implications of context transfer
- Context validation before execution

## Implementation Plan (Tentative)

### Phase 1: Research & Exploration

- Complete all research tickets (FD-TRANSFER, SOCKET-DISCOVERY, DAEMON-LIFECYCLE, EXECUTION-CONTEXT)
- Build minimal prototype of fd transfer using `sendfd`
- Test socket discovery strategies
- Experiment with daemonization patterns

### Phase 2: Daemon Foundation

- Basic daemon structure with double-fork
- PID file management
- Unix domain socket server
- Command execution with transferred descriptors
- Single command: `add` (sum all arguments, complain on stderr)

### Phase 3: Client Implementation

- Minimal client binary
- Socket discovery implementation
- Context serialization and transfer
- Auto-launch daemon via --create flag

### Phase 4: Enhancement

- XDG-project mode support
- sd-notify integration on descriptor 4
- Client miniaturization
- Comprehensive error handling

## Short-Names

- **FD-TRANSFER**: File descriptor transfer mechanisms
- **SOCKET-DISCOVERY**: Socket discovery strategies
- **DAEMON-LIFECYCLE**: Process lifecycle management
- **EXECUTION-CONTEXT**: Execution context serialization
- **PFD**: PreFork Daemon
- **PFC**: PreFork Client
- **XDG-PATH**: XDG compliant directory paths

## Open Questions

1. Should PFD return execution status to PFC, or terminate independently?
2. How to handle concurrent executions - serialize or parallel?
3. What's the maximum descriptor count we should support?
4. Should we support passing environment variables?
5. How to handle signals during execution (SIGINT, SIGTERM)?

## Related Technologies

- **Rust**: Systems programming language
- **sendfd**: File descriptor passing over Unix sockets
- **fork**: Process forking utilities
- **directories**: XDG directory resolution
- **Unix Domain Sockets**: IPC mechanism
- **Ancillary Data**: Socket-level metadata for fd passing

## Success Criteria

- Client successfully transfers execution context to daemon
- Daemon executes commands with transferred descriptors
- Multiple discovery strategies work reliably
- Daemon runs in background and responds to signals
- Clean shutdown with PID file cleanup
