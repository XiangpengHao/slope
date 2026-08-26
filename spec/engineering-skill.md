
- use descriptive, intention-revealing names for variables and functions, don't use generic names like `Built`, `Common`, `View`.

- do not use free form functions, functions should have a clear owner, e.g., struct, enum; unless it is fully justified stateless function.
  Example 1: `fn build_chart(model: &DataModel) -> Built`, instead use a member method like: `fn build_chart(&self) -> Built`. Even better, use a `From` trait to build the chart from the data model.
  Example 2: `fn toggle_expand(code: crate::views::codemap::CodeState, key: (u32, u32))` instead use a member method like: `fn toggle_expand(&self, key: (u32, u32))`.

- try to use minimal visibility for functions and variables, e.g., a pub function under a private mod is essentially private, but it is confusing, prefer to make it private. 

- try to avoid public fields, prefer to use getters and setters, unless the field is truly public.

- organize the code by features, not by technical buckets, e.g., don't have `api.rs`, `views.rs`, `data.rs`, etc. Instead, do `settings.rs`, `data_panel.rs`, `code_map.rs`, etc.

- No static variables.

- Don't use large unconstrained struct constructions, e.g., following code is ugly and error prone.
Maybe there're invariants we want to enforce, but this code allows any fields to be set.
```rust
ghost_nodes.push(CrateInfo {
                            id: ghost_id.clone(),
                            name: ev.name.clone(),
                            version: ev.detail.clone().unwrap_or_default(),
                            is_member: false,
                            changed: false,
                            changed_files: 0,
                            manifest_changed: false,
                            affected_dist: None,
                            dependents: 0,
                            direct_deps: 0,
                            external_deps: 0,
                            ghost: true,
                            description: None,
                            license: None,
                            repository: None,
                            homepage: None,
                            documentation: None,
                            // A removed dependency's manifest is gone with
                            // it; the name is all we know.
                            crates_io: false,
                            rel_path: None,
                        });
```
Instead, we use use a much narrower constructor, with new(arg1, arg2, arg3), and check and enforce the invariants.

- Typically a function should not have more than 3 parameters (including self if it is a member method). If it does, it is a sign of either too large function body, or a container struct should hold the parameters. 

- A struct should not have more than 7 fields, more than that adds cognitive burden. Use private structs to group related fields.

### How to refactor crate boundary

Frist, think from first principle and answer each of the following questions:
- What does this crate do? What should this crate do?
- What is the minimal information this crate needs to do its job?
- What are the preconditions and postconditions of this crate's input/output?

Then, assuming you can make breaking changes, answer:
- How to compose this information? Builder style? Public structs with getters and setters? Runtime dynamic steering?
- What is the data model for the information? How to represent data so that illegal states are not possible?
- Where to check the preconditions? In the constructor? How to make sure preconditions are always held for all valid representations?
- How to test postconditions? Is it testable?
- How does ergnomic Rust code look like? What traits should be implemented?

Finally, do the refactoring:
- Refactoring should not be mechanical, for every breaking change, ask what should stay in the app verus move to the crate?
- A crate boundary exists because the boundary is much narrower than the details, ow. we'd just inline the crate details into the app.

