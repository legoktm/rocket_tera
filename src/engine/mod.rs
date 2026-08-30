use std::path::Path;
use std::collections::HashMap;

use rocket::serde::Serialize;

use crate::template::TemplateInfo;

mod tera;
use ::tera::Tera;

pub(crate) trait Engine: Send + Sync + Sized + 'static {
    const EXT: &'static str;

    fn init<'a>(templates: impl Iterator<Item = (&'a str, &'a Path)>) -> Option<Self>;
    fn render<C: Serialize>(&self, name: &str, context: C) -> Option<String>;
}

/// A structure exposing access to templating engines.
///
/// Calling methods on the exposed template engine types may require importing
/// types from the respective templating engine library. These types should be
/// imported from the reexported crate at the root of `rocket_dyn_templates` to
/// avoid version mismatches. For instance, when registering a Tera filter, the
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
    pub(crate) const ENABLED_EXTENSIONS: &'static [&'static str] = &[
        Tera::EXT,
    ];

    pub(crate) fn init(templates: &HashMap<String, TemplateInfo>) -> Option<Engines> {
        fn inner<E: Engine>(templates: &HashMap<String, TemplateInfo>) -> Option<E> {
            let named_templates = templates.iter()
                .filter(|&(_, i)| i.engine_ext == E::EXT)
                .filter_map(|(k, i)| Some((k.as_str(), i.path.as_ref()?)))
                .map(|(k, p)| (k, p.as_path()));

            E::init(named_templates)
        }

        Some(Engines {
            tera: inner::<Tera>(templates)?,
        })
    }

    pub(crate) fn render<C: Serialize>(
        &self,
        name: &str,
        info: &TemplateInfo,
        context: C,
    ) -> Option<String> {
        if info.engine_ext == Tera::EXT {
            return Engine::render(&self.tera, name, context);
        }

        None
    }

    /// Returns iterator over template (name, engine_extension).
    pub(crate) fn templates(&self) -> impl Iterator<Item = (&str, &'static str)> {
        self.tera.get_template_names().map(|name| (name, Tera::EXT))
    }
}
