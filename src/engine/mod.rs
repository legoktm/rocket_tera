use std::collections::HashMap;

use rocket::serde::Serialize;

use crate::template::TemplateInfo;

mod tera;
use ::tera::Tera;

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
        let named_templates = templates.iter()
            .filter_map(|(k, i)| Some((k.as_str(), i.path.as_ref()?)))
            .map(|(k, p)| (k, p.as_path()));

        Some(Engines { tera: tera::init(named_templates)? })
    }

    pub(crate) fn render<C: Serialize>(&self, name: &str, context: C) -> Option<String> {
        tera::render(&self.tera, name, context)
    }

    /// Returns an iterator over the names of registered templates.
    pub(crate) fn templates(&self) -> impl Iterator<Item = &str> {
        self.tera.get_template_names()
    }
}
