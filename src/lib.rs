//! Tera templating support for Rocket.
//!
//! This crate adds support for using [Tera](https://keats.github.io/tera/) with Rocket. It
//! automatically discovers templates, provides a `Responder` to render templates,
//! and automatically reloads templates when compiled in debug mode.
//!
//! # Usage
//!
//!   1. Depend on the crate: `cargo add rocket_tera`
//!
//!   2. Write your templates inside of the [configurable]
//!      `${ROCKET_ROOT}/templates`. Every file in that directory is a template;
//!      no particular extension is required, but the extension determines the
//!      response's `Content-Type`, as in
//!      `${ROCKET_ROOT}/templates/index.html`.
//!
//!      [configurable]: #configuration
//!      [Tera]: https://docs.rs/crate/tera/2
//!
//!   3. Attach `Template::fairing()` and return a [`Template`] from your routes
//!      via [`Template::render()`], supplying the path of the template file
//!      **relative to the template directory**:
//!
//!      ```rust
//!      # #[macro_use] extern crate rocket;
//!      use rocket_tera::{Template, context};
//!
//!      #[get("/")]
//!      fn index() -> Template {
//!          Template::render("index.html", context! { field: "value" })
//!      }
//!
//!      #[launch]
//!      fn rocket() -> _ {
//!          rocket::build().attach(Template::fairing())
//!      }
//!      ```
//!
//! ## Configuration
//!
//! This crate reads one configuration parameter from the configured figment:
//!
//!   * `template_dir` (**default: `templates/`**)
//!
//!     A path to a directory to search for template files in. Relative paths
//!     are considered relative to the configuration file, or there is no file,
//!     the current working directory.
//!
//! For example, to change the default and set `template_dir` to different
//! values based on whether the application was compiled for debug or release
//! from a `Rocket.toml` file (read by the default figment), you might write:
//!
//! ```toml
//! [debug]
//! template_dir = "static/templates"
//!
//! [release]
//! template_dir = "/var/opt/www/templates"
//! ```
//!
//! **Note:** `template_dir` defaults to `templates/`. It _does not_ need to be
//! specified if the default suffices.
//!
//! See the [configuration chapter] of the guide for more information on
//! configuration.
//!
//! [configuration chapter]: https://rocket.rs/guide/v0.5/configuration/
//!
//! ## Template Naming and Content-Types
//!
//! Templates are rendered by _name_ via [`Template::render()`], which returns a
//! [`Template`] responder. The _name_ of the template is the path to the
//! template file, relative to `template_dir`, extension included.
//!
//! The `Content-Type` of the response is automatically determined by the
//! extension using [`ContentType::from_extension()`]. If there is no extension
//! or it is unknown, `text/plain` is used.
//!
//! The following table contains examples:
//!
//! | template path                             | [`Template::render()`] call            | content-type |
//! |-------------------------------------------|----------------------------------------|--------------|
//! | {template_dir}/index.html                 | `render("index.html")`                 | HTML         |
//! | {template_dir}/index                      | `render("index")`                      | `text/plain` |
//! | {template_dir}/dir/index                  | `render("dir/index")`                  | `text/plain` |
//! | {template_dir}/dir/data.json              | `render("dir/data.json")`              | JSON         |
//! | {template_dir}/data.template.xml          | `render("data.template.xml")`          | XML          |
//! | {template_dir}/subdir/index.template.html | `render("subdir/index.template.html")` | HTML         |
//!
//! Give every template the extension of the file type it renders to, so that
//! the `Content-Type` is correct: `.html`, `.xml`, and so on.
//!
//! [`ContentType::from_extension()`]: ../rocket/http/struct.ContentType.html#method.from_extension
//!
//! ### Rendering Context
//!
//! In addition to a name, [`Template::render()`] requires a context to use
//! during rendering. The context can be any [`Serialize`] type that serializes
//! to an `Object` (a dictionary) value. The [`context!`] macro can be used to
//! create inline `Serialize`-able context objects.
//!
//! [`Serialize`]: rocket::serde::Serialize
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use rocket::serde::Serialize;
//! use rocket_tera::{Template, context};
//!
//! #[get("/")]
//! fn index() -> Template {
//!     // Using the `context! { }` macro.
//!     Template::render("index.html", context! {
//!         site_name: "Rocket - Home Page",
//!         version: 127,
//!     })
//! }
//!
//! #[get("/")]
//! fn index2() -> Template {
//!     #[derive(Serialize)]
//!     #[serde(crate = "rocket::serde")]
//!     struct IndexContext {
//!         site_name: &'static str,
//!         version: u8
//!     }
//!
//!     // Using an existing `IndexContext`, which implements `Serialize`.
//!     Template::render("index.html", IndexContext {
//!         site_name: "Rocket - Home Page",
//!         version: 127,
//!     })
//! }
//! ```
//!
//! ### Discovery, Automatic Reloads, and Engine Customization
//!
//! As long as one of [`Template::fairing()`], [`Template::custom()`], or
//! [`Template::try_custom()`] is [attached], every file in the configured
//! `template_dir` is registered with Tera and can be rendered. Files that are
//! not valid Tera templates will abort the launch, so keep non-template assets
//! out of `template_dir`.
//!
//! [`Template::custom()`] takes two callbacks, which run on either side of
//! template loading:
//!
//!   1. `register` to add filters, functions, and tests
//!   2. then the templates in `template_dir` are loaded,
//!   3. `finalize` allows adding additional templates
//!
//! _**Note:** Templates that are registered directly via [`Template::custom()`]
//! use whatever name was provided during that registration._
//!
//! ```rust
//! # #[macro_use] extern crate rocket;
//! use rocket_tera::Template;
//! use rocket_tera::tera::{Kwargs, State};
//!
//! fn shout(value: &str, _: Kwargs, _: &State) -> String {
//!     value.to_uppercase()
//! }
//!
//! #[launch]
//! fn rocket() -> _ {
//!     rocket::build().attach(Template::custom(
//!         |tera| tera.register_filter("shout", shout),
//!         |tera| {
//!             tera.add_raw_template("greeting.html", "{{ name | shout }}")
//!                 .expect("valid Tera template");
//!         },
//!     ))
//! }
//! ```
//!
//! In debug mode (without the `--release` flag passed to `cargo`), templates
//! are **automatically reloaded** from disk when changes are made. In release
//! builds, template reloading is disabled to improve performance and cannot be
//! enabled.
//!
//! [attached]: rocket::Rocket::attach()
//!
//! ### Metadata and Rendering to `String`
//!
//! The [`Metadata`] request guard allows dynamically querying templating
//! metadata, such as whether a template is known to exist
//! ([`Metadata::contains_template()`]), and to render templates to `String`
//! ([`Metadata::render()`]).

#[macro_use]
extern crate rocket;

#[doc(inline)]
/// The tera templating engine library, reexported.
pub use tera;

#[doc(hidden)]
pub use rocket::serde;

mod context;
mod engine;
mod fairing;
mod metadata;
mod template;

pub use metadata::Metadata;
pub use template::Template;
