# seruq-test-runtime

An experimental library for rendering validated Seruq UCR documents. It provides
a CPU renderer and a desktop window host while the official `seruq-api` is still
being designed. Its public API and presentation choices are provisional.

## Library usage

The application supplies a validated `seruq_ucr::Ucr` and a base directory for
relative image and font paths. Decoding and validation belong to the caller;
the library does not depend on `seruq-ucr-json`.

```rust,ignore
use seruq_test_runtime::{Runtime, WindowOptions};

// `document` is a validated seruq_ucr::Ucr.
let mut runtime = Runtime::new(document, "/path/to/assets");
let frame = runtime.render(640, 480, 1.0)?;
frame.save_png("preview.png")?;
runtime.run(WindowOptions::default())?;
```

`render` takes physical pixel dimensions and a logical-to-physical scale factor.
It needs no window or display server. `Frame` exposes its dimensions, RGBA bytes,
and PNG export. `run` consumes the runtime, opens a window, and blocks until it
closes or an error occurs; call it on the main thread.

`layout` returns every reachable element in paint order with its logical bounds
and the region left after clipping by its ancestors. Rendering and `hit_test`
both derive from this list. `hit_test` takes a logical point and returns the
frontmost element under it; Containers are never hit, and every other element
type hides what is behind it. `apply` passes a mutation to the document, which
either applies all of its operations or leaves the document unchanged.

`handle_input` takes pointer input in logical coordinates and returns the UCR
events it produces, without a window:

| Input | Events |
| --- | --- |
| Press on a Button | `Pushed` |
| Release over the same Button | `Released`, then `Activated` |
| Release anywhere else | `Released` |
| `PointerCancelled` during a press | `Released` |

Only Buttons respond, only one press is in progress at a time, and moving the
pointer produces no events. A Button hidden by another element or clipped away
at the pointer's position does not respond. `pressed` returns the Button that
is held down with the pointer over it; it is drawn with a gray background.
`apply` drops a press whose Button the mutation removed.

The runtime does not call functions. `run_with` opens a window and passes each
event to a handler, which typically resolves it with `seruq-ffr` and applies the
returned mutation; the window is redrawn afterwards. `run` discards events.

```rust,ignore
runtime.run_with(WindowOptions::default(), move |runtime, event| {
    if let Ok(Some(mutation)) = seruq_ffr::call(&mut functions, runtime.document(), &event) {
        let _ = runtime.apply(&mutation);
    }
})?;
```

`run_with_events` also delivers `RuntimeEvent::ViewportResized` with logical
width, height, and scale factor. The window host delivers the initial viewport
before the first frame and changes before the next frame, suppressing identical
notifications. The application can apply layout mutations in the callback; the
runtime never resizes the document's root implicitly. Zero dimensions represent
a minimized window; rendering waits until both physical dimensions are nonzero.
A newly created window after suspension receives a fresh initial notification.

Offscreen hosts call `resize(physical_width, physical_height, scale_factor)` and
handle its optional event before calling `render`. Invalid scales leave the
previous viewport unchanged. `render` itself does not generate viewport events.
`run_with` remains available for element events alone. Viewport callbacks belong
to the host for now; there is no document-level FFR action in the JSON format.
The `interactive_counter` example uses viewport events to wrap its buttons.

The window host is behind the default `window` feature. Disable default
features to use rendering, layout, and hit testing without winit and softbuffer.

## Examples

Run these commands from the workspace root. The examples are clients of the
library, not part of its public API.

```sh
# Load the bundled JSON scene and open a window.
cargo run -p seruq-test-runtime --example viewer

# Load a JSON document and export a PNG without opening a window.
cargo run -p seruq-test-runtime --example viewer -- crates/seruq-test-runtime/examples/scene.json /tmp/seruq-preview.png

# Open a counter whose Buttons call functions through FFR.
cargo run -p seruq-test-runtime --example interactive_counter

# Simulate clicks on the counter and export a PNG without opening a window.
cargo run -p seruq-test-runtime --example interactive_counter -- /tmp/seruq-counter.png
```

The JSON viewer resolves relative asset paths against the document's directory.
It reports validation warning counts before rendering. Without an output path,
it opens a 640 by 480 logical-unit window.

## Current behavior

- `Container`: transparent; positions are relative to the parent, descendants
  are clipped to its bounds, and children are painted in array order.
- `Text`: black text, explicit newlines, no automatic wrapping.
- `Image`: PNG files, scaled to fit while preserving aspect ratio and centered
  inside the element.
- `Button`: a white background, gray while pressed, and a black border, with a
  centered label.
- `TextInput`: the same border with a value or gray placeholder; only the first
  line is displayed.

The viewport is white. Leaf content is clipped to its own bounds as well as its
ancestors. A text element's font overrides `metadata.default_font`; without
either, this runtime uses installed system fonts. Explicit font files use their
first face, without system-font fallback. Assets are loaded lazily and cached
for the runtime's lifetime, so changes to those files are not reloaded.

Buttons respond to the primary pointer button. Text inputs are static visuals.
Keyboard input, focus, editing, and touch are not implemented.
While a window is running, a mutation can only be applied from the event handler. Visible `Geometry` elements return an
`UnsupportedElement` error because the current UCR model has no shape data.
Fully clipped elements are skipped, including their asset loading. Renderer
coordinate limits can also produce an error for otherwise valid UCR documents.

Winit manages the window and event loop; softbuffer presents CPU pixels;
tiny-skia draws shapes and PNGs; cosmic-text shapes and rasterizes text. Window
redraws use the OS scale factor and occur on demand, including after resize and
scale changes.
