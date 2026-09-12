use std::error::Error;
use std::path::Path;

use rocket::serde::Serialize;
use tera::{Context, Tera};

/// Initializes `Tera` instance.
pub(crate) fn init() -> Tera {
    let mut tera = Tera::default();
    tera.autoescape_on([".html", ".htm", ".xml"]);
    crate::contrib::register(&mut tera);
    tera
}

/// Registers every file in `root` with `tera`, named by its path relative to
/// `root`. Tera remembers the glob so the templates can be reloaded later.
pub(crate) fn load(tera: &mut Tera, root: &Path) -> Option<()> {
    let glob = root.join("**").join("*");
    let Some(glob) = glob.to_str() else {
        error_!(
            "Template directory '{}' is not valid UTF-8.",
            root.display()
        );
        return None;
    };

    if let Err(e) = tera.load_from_glob(glob) {
        error_!("Tera templating initialization failed.");
        info_!("{}", e);
        log_error(&e);
        return None;
    }

    Some(())
}

/// Reloads every template previously loaded by [`load()`] from disk. If that
/// fails, `tera` keeps its existing templates.
#[cfg(debug_assertions)]
pub(crate) fn reload(tera: &mut Tera) -> Option<()> {
    if let Err(e) = tera.full_reload() {
        error_!("Tera template reloading failed.");
        info_!("{}", e);
        log_error(&e);
        return None;
    }

    Some(())
}

pub(crate) fn render<C: Serialize>(tera: &Tera, template: &str, context: C) -> Option<String> {
    if !tera.contains_template(template) {
        error_!("Tera template '{}' does not exist.", template);
        return None;
    };

    let tera_ctx = Context::from_serialize(&context)
        .map_err(|e| error_!("Tera context error: {}.", e))
        .ok()?;

    match tera.render(template, &tera_ctx) {
        Ok(string) => Some(string),
        Err(e) => {
            error_!("Error rendering Tera template '{}': {}", template, e);
            log_error(&e);
            None
        }
    }
}

/// Logs the `source()` chain of `error`, which Tera uses to carry the
/// underlying cause of I/O and (de)serialization failures.
pub(crate) fn log_error(error: &dyn Error) {
    let mut source = error.source();
    while let Some(err) = source {
        info_!("{}", err);
        source = err.source();
    }
}
