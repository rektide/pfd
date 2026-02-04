# prefork-rs Code Review

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        pfc (Client)                              │
│  CLI Parser → ExecutionContext → sendfd Transfer                │
└──────────────────────────┬──────────────────────────────────────┘
                           │ Unix Datagram + FDs
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                        pfd (Daemon)                              │
│  Unix Socket Receiver → rkyv Deserialize → CmdRegistry → add   │
└─────────────────────────────────────────────────────────────────┘
                           │
              ┌────────────┴────────────┐
              ▼                         ▼
        discovery/                  context/
   (socket discovery)         (ExecutionContext)
```

---

## Critical Issues

### 1. Duplicate Code in xdg_runtime_create.rs

**File:** `discovery/src/xdg_runtime_create.rs`

The file contains duplicate impl blocks (lines 6-45 vs 66-98):
- Duplicate `impl Default`
- Duplicate `impl XdgRuntimeCreateStrategy`
- Duplicate `impl CreateStrategy`
- Duplicate `#[cfg(test)]` module

The second half references `UserDirs::new()` without importing it. This either won't compile or has dead code.

**Fix:** Remove the duplicate block (lines 66-117) and keep the first implementation using `XDG_RUNTIME_DIR` env var.

### 2. Wildcard Dependency Versions

**File:** `Cargo.toml` (lines 12-21)

```toml
clap = { version = "*", ... }
anyhow = "*"
thiserror = "*"
tokio = { version = "*", ... }
```

Using `*` breaks reproducibility and can pull semver-incompatible changes.

**Fix:** Pin specific versions in workspace dependencies.

### 3. Daemon Receive Loop Anti-Pattern

**File:** `pfd/src/lib.rs` (lines 131-176)

Current implementation uses:
- `spawn_blocking` per packet
- `Arc<Mutex<std::net::UnixDatagram>>`
- Manual `sleep(10ms)` on `WouldBlock`

This creates unnecessary tasks, lock contention, and CPU spin.

**Fix:** Replace with `tokio::io::unix::AsyncFd` around the std socket and call `recv_with_fd` inside `try_io`.

---

## Security Concerns

### High Priority

1. **Socket Placement + Permissions**
   - Binding in `./` is risky
   - `std::fs::remove_file(&socket_path).ok()` before bind can delete arbitrary files
   - **Fix:** Use `$XDG_RUNTIME_DIR` with directory permissions `0700`

2. **Unauthenticated Local IPC**
   - Any process that can reach the socket can submit commands
   - No peer credential checking
   - **Mitigation:** Enforce socket only accessible to same UID via filesystem permissions

3. **rkyv and Untrusted Bytes**
   - Must ensure validated deserialization APIs are used
   - Currently using `from_bytes::<ExecutionContext, Error>` which should validate with bytecheck

### Medium Priority

4. **DoS Vectors**
   - No rate limiting / concurrency limits
   - No payload size limit (buffer is fixed 16384, truncation causes corrupted data)
   - No per-command timeout

---

## Code Quality Issues

### CmdRegistry Cloning

**File:** `pfd/src/lib.rs` (lines 48-52, 154)

```rust
fn clone_ref(&self) -> Self {
    Self {
        commands: self.commands.clone(),
    }
}
```

This clones the entire `HashMap` on each request. Values are `Arc` but map clone still allocates.

**Fix:** Use `Arc<CmdRegistry>` and clone the Arc instead.

### Verbose Flag Not Wired

**File:** `pfd/src/main.rs` (lines 12-14, 18-21)

The `--verbose` flag exists but isn't used—filtering always uses `EnvFilter::from_default_env()`.

### XDG Strategies Not Integrated

**File:** `discovery/src/lib.rs` (lines 136-157)

`discover_socket()` only uses:
1. CLI argument
2. `PFD_SOCKET` env var
3. `LocalFileStrategy`

It never consults `XdgRuntimeStrategy`, contrary to README claims.

### FD Handling

**File:** `pfd/src/lib.rs` (lines 72-73, 88-90, 101-103)

Repeated `unsafe` conversions: `OwnedFd → raw → File::from_raw_fd`.

**Fix:** Prefer safe conversions: `std::fs::File::from(OwnedFd)` then `tokio::fs::File::from_std`.

### Client Blocking in Async

**File:** `pfc/src/lib.rs` (lines 49)

`send_with_fd` is synchronous but called in async context. Can block runtime thread in pathological conditions.

---

## Missing README TODOs

From README.md "Todo" section, not yet implemented:

| Feature | Status |
|---------|--------|
| Auto-launch daemon (`--create/-C`) | Missing |
| Discovery via pidfile in XDG locations | Missing |
| XDG++ app-name support (`${XDG_RUNTIME_DIR}/pfd.<app>.sock`) | Missing |
| `sd-notify` support on descriptor 4 | Missing |
| Miniaturize client | Client pulls clap, tracing, serde_json |

---

## Testing Coverage Gaps

### Meaningless Tests

**File:** `discovery/src/xdg_runtime.rs` (lines 42-52)

```rust
assert!(result.is_none() || result.is_some());  // Always true
assert_eq!(strategy.discover(), strategy.discover());  // Tautology
```

### Missing Integration Tests

- Real FD transfer (stdin/stdout/stderr)
- End-to-end client→daemon dispatch
- Socket discovery precedence (CLI vs env vs filesystem)
- Payload size rejection / invalid data rejection

### Test Race Conditions

Environment variable tests use `unsafe set_var/remove_var` which can race in parallel test execution.

---

## Dependency Management

### Inconsistent Workspace Usage

| Crate | Uses workspace deps? |
|-------|---------------------|
| pfc | Yes |
| pfd | No (pins explicit versions) |
| discovery | Yes |
| context | No (pins explicit versions) |

**Fix:** Use `dep.workspace = true` consistently in all member crates.

---

## Recommended Fixes by Effort

### Small (< 1 hour)

- [ ] Delete duplicate code in `xdg_runtime_create.rs` (lines 66-117)
- [ ] Pin dependency versions (remove `*`)
- [ ] Wire verbose flag in pfd
- [ ] Fix meaningless tests
- [ ] Normalize workspace dependency usage

### Medium (1-3 hours)

- [ ] Replace daemon receive loop with `AsyncFd`
- [ ] Safe socket cleanup (verify file is socket before removal)
- [ ] Integrate XDG discovery strategies into `discover_socket()`
- [ ] Add payload size checks + protocol header (magic bytes, version, length)
- [ ] Use `Arc<CmdRegistry>` instead of cloning

### Large (1-2 days)

- [ ] End-to-end integration tests with real FD transfer
- [ ] Authentication / peer credential checking
- [ ] Background launch / pidfiles / sd-notify
- [ ] Stream socket protocol (if larger payloads needed)

---

## Risks and Guardrails

1. **If pfd ever runs with higher privileges**, current model is privilege-escalation risk
   - Keep same-UID only via private runtime dir + strict perms
   - Reject requests unless peer UID matches
   - Do not implement "execute arbitrary command" without allowlist

2. **Serialization safety**: treat bytes as hostile; ensure rkyv validation and protocol versioning

3. **Datagram truncation**: enforce length checks; reject oversize payloads
