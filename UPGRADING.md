# `rocket_tera` 1.0 to 2.0

The main change in this release is upgrading to [`tera 2.0`](https://keats.github.io/tera/). You'll need to review the [upstream migration guide](https://github.com/Keats/tera/blob/master/MIGRATION.md) to adjust your templates, and any other code that uses tera APIs.

Filters, functions and tests that relied on other dependencies were moved into a separate `tera-contrib` crate. You will need to enable the corresponding `contrib-{name}` feature in this crate to be able to use
that functionality again.

Tera also changes the initalization order, which required changes to `Template::custom()` and `try_custom()`. Filters, functions, etc. must be registered before templates, so `custom()` now takes two callbacks: `register` and `finalize`. The loading order is:

1) `register` callback, for filters, functions, etc.
2) loading all templates in `template_dir`.
3) `finalize` callback, if you want to add any more templates.

# `rocket_dyn_templates` 0.2 to `rocket_tera` 1.0

First adjust all `rocket_dyn_templates` references to `rocket_tera`, including in Cargo.toml.

Most importantly, templates no longer need a `.tera` suffix. Additionally, templates are named using their full path, so you'll now `render("index.html", ...)`. Any `{% extends %}` / `{% include %}` will also need to be updated.

`Template::custom` now receives a `Tera` object directly instead of `Engines` (which has been removed).

Finally MSRV is now Rust 1.88.
