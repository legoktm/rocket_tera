use std::collections::HashMap;
use std::error::Error;

use rocket::serde::Serialize;
use tera::{Context, Tera};

use crate::template::TemplateInfo;

/// The file extension identifying a template.
pub(crate) const EXT: &str = "tera";

/// Builds a `Tera` instance with every discovered template registered.
pub(crate) fn init(templates: &HashMap<String, TemplateInfo>) -> Option<Tera> {
    let mut tera = Tera::default();
    let ext = [
        ".html.tera",
        ".htm.tera",
        ".xml.tera",
        ".html",
        ".htm",
        ".xml",
    ];
    tera.autoescape_on(ext.to_vec());

    // Collect into a tuple of (path, name) for Tera. If we register one at
    // a time, it will complain about unregistered base templates.
    let files = templates
        .iter()
        .filter_map(|(name, info)| Some((info.path.as_ref()?, Some(name.as_str()))));

    // Finally try to tell Tera about all of the templates.
    if let Err(e) = tera.add_template_files(files) {
        error_!("Tera templating initialization failed.");
        let mut error = Some(&e as &dyn Error);
        while let Some(err) = error {
            info_!("{}", err);
            error = err.source();
        }

        return None;
    }

    Some(tera)
}

pub(crate) fn render<C: Serialize>(tera: &Tera, template: &str, context: C) -> Option<String> {
    if tera.get_template(template).is_err() {
        error_!("Tera template '{}' does not exist.", template);
        return None;
    };

    let tera_ctx = Context::from_serialize(context)
        .map_err(|e| error_!("Tera context error: {}.", e))
        .ok()?;

    match tera.render(template, &tera_ctx) {
        Ok(string) => Some(string),
        Err(e) => {
            error_!("Error rendering Tera template '{}': {}", template, e);
            let mut error = e.source();
            while let Some(err) = error {
                error_!("{}", err);
                error = err.source();
            }

            None
        }
    }
}
