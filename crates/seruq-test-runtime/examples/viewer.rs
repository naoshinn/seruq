use std::{error::Error, io, path::PathBuf};

use seruq_test_runtime::{Runtime, WindowOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/scene.json"));
    let output = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: viewer [document.json] [output.png]",
        )
        .into());
    }

    let draft = seruq_ucr_json::from_str(&std::fs::read_to_string(&path)?)?;
    let (document, warnings) = draft.build().map_err(|report| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "UCR validation failed: {} errors, {} warnings",
                report.errors().len(),
                report.warnings().len()
            ),
        )
    })?;
    if !warnings.is_empty() {
        eprintln!("UCR validation: {} warnings", warnings.len());
    }

    // Choosing the document's directory as the asset base is this example's policy.
    let mut runtime = Runtime::new(
        document,
        path.parent().unwrap_or_else(|| std::path::Path::new(".")),
    );
    let options = WindowOptions::default();
    if let Some(output) = output {
        runtime
            .render(options.width, options.height, 1.0)?
            .save_png(&output)?;
        println!("Saved {}", output.display());
    } else {
        runtime.run(options)?;
    }
    Ok(())
}
