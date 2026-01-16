# pre-fork-democli

> CLI Demo for rust preforking

Client/server

- **`pfd`** - PreFork Cli Demo Daemon, which receives executions from the client
- **`pfc`** - PreFork Cli Demo Client, which transfers its execution to the daemon

# Execution Context

What is an execution? What do we transfer?

| **context** | **description** |
| command | the program name to execute |
| args | array of arguments |
| cwd | current working directory |
| descriptors | array of file descriptors, starting with stdin / stderr / stdout |

Once exection is transfered, all resources are in the daemon's hands, and the client terminates.

# `pfc` - Small Client Philosophy

We wish to keep the client incredibly small and simple. It's goal is to find the daemon quickly, transfer the execution context, and close.

# `pfd` - Just a Demo server

To start, pfcdd only has one command:

| command | description |
| add | add all arguments. complain on stderr. |

# Fd Transfer

## Discovery / Rendezvous

The heart of `pfd` is a sending file descriptors. We send via a socket opened by the daemon.

Client has to be able to send it's descriptors. It first finds a unix domain socket, then transfers the execution context & it's file descriptors over it.

User can pick among strategies selected from to find the socket to communicate over:

1. `./pfd.sock` mode, look right here. Also accepts `.pfd.sock`.
2. xdg mode, using user runtime dir for pdf.sock
3. xdg-project mode, which has a project name middle-fix for the socket, eg `${XDG_RUNTIME_DIR}/pfd.pfd-demo.sock`

# Todo

- Auto-launch daemon into background (`--create/-C` option)
- Discovery, look for pid (process id) file in XDG compliant locations.
- "XDG"++ app-name support
- sd-notify support on descriptor 4.
- miniaturize client even more
