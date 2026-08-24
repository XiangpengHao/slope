
- use descriptive names for variables and functions, don't use generic names like `Built`, `Common`, `View`.

- try to not use free form functions, perfer to use member methods.
  Example 1: `fn build_chart(model: &DataModel) -> Built`, instead use a member method like: `fn build_chart(&self) -> Built`. Even better, use a `From` trait to build the chart from the data model.
  Example 2: `fn toggle_expand(code: crate::views::codemap::CodeState, key: (u32, u32))` instead use a member method like: `fn toggle_expand(&self, key: (u32, u32))`.

- try to use minimal visibility for functions and variables, e.g., a pub function under a private mod is essentially private, but it is confusing, prefer to make it private. 

- try to avoid public fields, prefer to use getters and setters, unless the field is truly public.
