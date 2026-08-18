we're building slopify.

the backend reads a cargo manifest, reads the dependencies and builds a dependency graph.

## Frontend

Frontend is ui heavy.
The first view is a dependency graph viewer.
For every crate, it shows what crates it depends on and what crates depend on it.
