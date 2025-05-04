```rust
# use ferrosaur::js;
#
#[js(module("../examples/js/mod.js"))]
struct Module;

#[js(interface)]
impl Module {
    #[js(prop)]
    fn todos(&self) -> TodoList {}
}

#[js(value)]
struct TodoList;

#[js(interface)]
impl TodoList {
    #[js(func)]
    fn create(&self) -> Todo {}
}

#[js(value)]
struct Todo;

#[js(interface)]
impl Todo {
    #[js(prop(with_setter))]
    fn done(&self) -> bool {}
}
```
