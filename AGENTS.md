We're building a rust workspace viewer, to help user review the code changes made by llm.

- Don't edit this file.

- Iterate ./spec as we add new features, change behavior, etc.

- use simple tech english


## Project goals

A cli tool user points to a cargo workspace, it analyzes, opens a browser window to visualize the workspace.

### Dependency graph viewer
The first task is a dependency graph viewer.

The biggest challenge is that dependency graph is a high dimensional DAG.
We can easily have thousands of nodes and edges, and overwhelm user's cognitive load.
We need to think carefully about what's best for the user.

### Code structure viewer
The second task is a code structure viewer.
Code structure is a more complex and more dimensional graph (not even a DAG).
We want users to be able to navigate the code structures. It almost looks like a recursive graph, where you have crates relationships, files, structs, functions, etc.


## Tech stack

- It is a dioxus web app (backend and frontend web app all in one).
- tailwind css for styling
- browser is avaliable through nix packages.
- [dioxus-flow](https://github.com/XiangpengHao/dioxus-flow) for the graph viewer, check doc carefully.

