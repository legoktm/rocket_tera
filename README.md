# `rocket_tera` [![crates.io]][crate]

[crates.io]: https://img.shields.io/crates/v/rocket_tera.svg
[crate]: https://crates.io/crates/rocket_tera

This crate adds support for using [Tera](https://keats.github.io/tera/) with Rocket. It
automatically discovers templates, provides a `Responder` to render templates,
and automatically reloads templates when compiled in debug mode.

# Usage

  1. Write your template files in the configurable `template_dir` directory (default:
     `{rocket_root}/templates`).

  2. Attach `Template::fairing()` and return a `Template` using
     `Template::render()`, supplying the name of the template file **minus the
     last two extensions**:

     ```rust
     use rocket_tera::{Template, context};

     #[get("/")]
     fn index() -> Template {
         Template::render("template-name", context! { field: "value" })
     }

     #[launch]
     fn rocket() -> _ {
         rocket::build().attach(Template::fairing())
     }
     ```

See the [crate docs](https://docs.rs/rocket_tera) for full details.

# History

This was originally forked from the `rocket_dyn_templates` crate, which was maintained as part of Rocket upstream.
