
- use descriptive names for variables and functions, don't use generic names like `Built`, `Common`, `View`.

- try to not use free form functions like: `fn build_chart(model: &DataModel) -> Built`, instead use a member function like: `fn build_chart(&self) -> Built`. Even better, use a `From` trait to build the chart from the data model.

- try to use minimal visibility for functions and variables, e.g., a pub function under a private mod is essentially private, but it is confusing, prefer to make it private. 

- try to avoid public fields, prefer to use getters and setters, unless the field is truly public.
