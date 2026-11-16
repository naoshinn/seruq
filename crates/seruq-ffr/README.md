# seruq-ffr

An experimental Foreign Function Runner for Seruq UCR. It connects the function
names stored in a document's actions to functions supplied by the application.

- `Functions` registers Rust closures under function names.
- `call` finds the action that an event's target has for the event's timing,
  runs its function, and returns the function's mutation.
- `unresolved` lists the actions of a document that name a function the host
  does not have, so that missing functions are found when a document is loaded.
- `FunctionHost` is the interface behind `call` and `unresolved`. Hosts for
  functions written in other languages can implement it later.

A function receives the event and a read-only view of the document, and returns
a `Mutation` or an error. It does not change the document. `call` does not apply
the mutation either: the application passes it to `Ucr::apply`, normally before
handling the next event, so that every function reads the current document.

```rust,ignore
let mut functions = Functions::new();
functions.register("counter.increment", |event: &Event, document: &Ucr| {
    let count = document.data().state["count"].as_number().ok_or("not a number")? + 1.0;
    Ok(vec![MutationOp::SetState {
        key: "count".into(),
        value: Number::new(count)?.into(),
    }])
});

if let Some(mutation) = call(&mut functions, &document, &event)? {
    document.apply(&mutation)?;
}
```

An event whose target is no longer reachable, or has no action for the event's
timing, calls nothing and yields `None`. An unknown function name and a function
failure are errors that identify the element, timing, and function name; neither
changes the document.

This crate depends only on `seruq-ucr`. It does not produce events; a runtime
does. Run the headless example from the workspace root:

```sh
cargo run -p seruq-ffr --example counter
```
