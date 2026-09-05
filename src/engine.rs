use std::collections::HashMap;
use std::error::Error;

use rocket::serde::Serialize;
use tera::{Context, Tera};

use crate::template::TemplateInfo;

/// Initializes `Tera` instance.
pub(crate) fn init() -> Tera {
    let mut tera = Tera::default();
    tera.autoescape_on([".html", ".htm", ".xml"]);
    crate::contrib::register(&mut tera);
    tera
}

/// Registers every discovered template with `tera`.
pub(crate) fn load(tera: &mut Tera, templates: &HashMap<String, TemplateInfo>) -> Option<()> {
    // Collect into a tuple of (path, name) for Tera. If we register one at
    // a time, it will complain about unregistered base templates.
    let files = templates
        .iter()
        .filter_map(|(name, info)| Some((info.path.as_ref()?, Some(name.as_str()))));

    // Finally try to tell Tera about all of the templates.
    if let Err(e) = tera.add_template_files(files) {
        error_!("Tera templating initialization failed.");
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
