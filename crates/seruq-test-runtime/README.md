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

## Examples

Run these commands from the workspace root. The examples are clients of the
library, not part of its public API.

```sh
# Load the bundled JSON scene and open a window.
cargo run -p seruq-test-runtime --example viewer

# Load a JSON document and export a PNG without opening a window.
cargo run -p seruq-test-runtime --example viewer -- crates/seruq-test-runtime/examples/scene.json /tmp/seruq-preview.png
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
- `Button`: a fixed white background and black border, with a centered label.
- `TextInput`: the same border with a value or gray placeholder; only the first
  line is displayed.

The viewport is white. Leaf content is clipped to its own bounds as well as its
ancestors. A text element's font overrides `metadata.default_font`; without
either, this runtime uses installed system fonts. Explicit font files use their
first face, without system-font fallback. Assets are loaded lazily and cached
for the runtime's lifetime, so changes to those files are not reloaded.

Buttons and text inputs are static visuals. Input events, focus, editing, FFR,
and mutations are not implemented. Visible `Geometry` elements return an
`UnsupportedElement` error because the current UCR model has no shape data.
Fully clipped elements are skipped, including their asset loading. Renderer
coordinate limits can also produce an error for otherwise valid UCR documents.

Winit manages the window and event loop; softbuffer presents CPU pixels;
tiny-skia draws shapes and PNGs; cosmic-text shapes and rasterizes text. Window
redraws use the OS scale factor and occur on demand, including after resize and
scale changes.
