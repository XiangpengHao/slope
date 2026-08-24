
1. use descriptive names for variables and functions, don't use generic names like `Built`, `Common`, `View`.

2. do not use free form functions like: `fn build_chart(model: &DataModel) -> Built`, instead use a member function like: `fn build_chart(&self) -> Built`. Even better, use a `From` trait to build the chart from the data model.

2. 