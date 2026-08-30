# `rocket_dyn_templates` 0.2 to `rocket_tera` 1.0

First adjust all `rocket_dyn_templates` references to `rocket_tera`, including in Cargo.toml.

Most importantly, templates no longer need a `.tera` suffix. Additionally, templates are named using their full path, so you'll now `render("index.html", ...)`. Any `{% extends %}` / `{% include %}` will also need to be updated.

The signature of `Template:custom` has changed and it's now named `customize`.

```diff
-.attach(Template::custom(|engines| {
-    engines.tera.register_filter("my_filter", my_filter);
+.attach(Template::customize(|tera| {
+    tera.register_filter("my_filter", my_filter);
 }))
```

Finally MSRV is now Rust 1.88.
