# prefork-democli-rs

> CLI Demo for rust preforking

Client/server

- **`pfcdd`** - PreFork Cli Demo Daemon, which receives executions from the client
- **`pfcdc`** - PreFork Cli Demo Client, which transfers its execution to the daemon

# Execution Context

What is an execution? What do we transfer?

| **context** | **description** |
| command | the program name to execute |
| args | array of arguments |
| cwd | current working directory |
| descriptors | array of file descriptors, starting with stdin / stderr / stdout |

Once exection is transfered, all resources are in the daemon's hands, and the client terminates.

# `pfcdc` - Small Client Philosophy

We wish to keep the client incredibly small and simple. It's goal is to find the daemon quickly, transfer the execution context, and close.

# `pfcdd` - Just a Demo server

To start, pfcdd only has one command:

| command | description |
| add | add all arguments. complain on stderr. |

# Todo

- Discovery, look for pid (process id) file in XDG compliant locations.
- "XDG"++ app-name support
- sd-notify support on descriptor 4.
- miniaturize client even more
