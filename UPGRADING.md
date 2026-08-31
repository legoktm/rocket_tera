# `rocket_dyn_templates` 0.2 to `rocket_tera` 1.0

First adjust all `rocket_dyn_templates` references to `rocket_tera`, including in Cargo.toml.

Most importantly, templates no longer need a `.tera` suffix. Additionally, templates are named using their full path, so you'll now `render("index.html", ...)`. Any `{% extends %}` / `{% include %}` will also need to be updated.

`Template::custom` now receives a `Tera` object directly instead of `Engines` (which has been removed).

Finally MSRV is now Rust 1.88.
