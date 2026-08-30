use std::collections::HashMap;
use std::error::Error;

use rocket::serde::Serialize;
use tera::{Context, Tera};

use crate::template::TemplateInfo;

/// The file extension identifying a template.
pub(crate) const EXT: &str = "tera";

/// A structure exposing access to the templating engine.
///
/// Calling methods on the exposed template engine type may require importing
/// types from the templating engine library. These types should be imported
/// from the reexported crate at the root of `rocket_dyn_templates` to avoid
/// version mismatches. For instance, when registering a Tera filter, the
/// [`tera::Value`] and [`tera::Result`] types are required. Import them from
/// `rocket_dyn_templates::tera`. The example below illustrates this:
///
/// ```rust
/// use std::collections::HashMap;
///
/// use rocket_dyn_templates::{Template, Engines};
/// use rocket_dyn_templates::tera::{self, Value};
///
/// fn my_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
///     # /*
///     ...
///     # */ unimplemented!();
/// }
///
/// fn main() {
///     rocket::build()
///         // ...
///         .attach(Template::custom(|engines: &mut Engines| {
///             engines.tera.register_filter("my_filter", my_filter);
///         }))
///         // ...
///         # ;
/// }
/// ```
///
/// [`tera::Value`]: crate::tera::Value
/// [`tera::Result`]: crate::tera::Result
pub struct Engines {
    /// A `Tera` templating engine.
    ///
    /// When calling methods on the `Tera` instance, ensure you use types
    /// imported from `rocket_dyn_templates::tera` to avoid version mismatches.
    pub tera: Tera,
}

impl Engines {
    pub(crate) fn init(templates: &HashMap<String, TemplateInfo>) -> Option<Engines> {
        // Create the Tera instance.
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
            span_error!("templating", "Tera templating initialization failed" => {
                let mut error = Some(&e as &dyn Error);
                while let Some(err) = error {
                    error!("{err}");
                    error = err.source();
                }
            });

            return None;
        }

        Some(Engines { tera })
    }

    pub(crate) fn render<C: Serialize>(&self, template: &str, context: C) -> Option<String> {
        if self.tera.get_template(template).is_err() {
            error!(template, "requested template does not exist");
            return None;
        };

        let tera_ctx = Context::from_serialize(context)
            .map_err(|e| error!("Tera context error: {}.", e))
            .ok()?;

        match self.tera.render(template, &tera_ctx) {
            Ok(string) => Some(string),
            Err(e) => {
                span_error!("templating", template, "failed to render Tera template" => {
                    let mut error = Some(&e as &dyn Error);
                    while let Some(err) = error {
                        error!("{err}");
                        error = err.source();
                    }
                });

                None
            }
        }
    }

    /// Returns an iterator over the names of registered templates.
    pub(crate) fn templates(&self) -> impl Iterator<Item = &str> {
        self.tera.get_template_names()
    }
}
