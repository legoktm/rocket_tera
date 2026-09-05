#[macro_use]
extern crate rocket;

#[cfg(test)]
mod tests;

use rocket::Request;
use rocket::response::Redirect;
use rocket_tera::tera::{Kwargs, State, Tera};
use rocket_tera::{Template, context};

/// Templates live next to this example, not in the current working directory,
/// so that `cargo run --example templating` works from anywhere in the repo.
const TEMPLATE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/templating/templates");

#[get("/")]
fn index() -> Redirect {
    Redirect::to(uri!(hello(name = "Your Name")))
}

#[get("/hello/<name>")]
fn hello(name: &str) -> Template {
    Template::render(
        "index.html",
        context! {
            title: "Hello",
            name: Some(name),
            items: vec!["One", "Two", "Three"],
        },
    )
}

#[get("/about")]
fn about() -> Template {
    Template::render("about.html", context! { title: "About" })
}

#[catch(404)]
fn not_found(req: &Request<'_>) -> Template {
    Template::render("error/404.html", context! { uri: req.uri() })
}

/// A custom filter, used by `index.html`. Filters must be registered before
/// the templates that use them are loaded, so this runs in `register`.
fn shout(value: &str, _: Kwargs, _: &State) -> String {
    value.to_uppercase()
}

/// Templates need not come from disk: this one is registered at startup, and is
/// re-registered on every reload in debug mode. It runs in `finalize` so that
/// `base.html`, loaded from disk, is already available to extend.
fn add_about_page(tera: &mut Tera) {
    tera.add_raw_template(
        "about.html",
        r#"
        {% extends "base.html" %}

        {% block content %}
            <section id="about">
              <h1>About - Here's another page!</h1>
            </section>
        {% endblock content %}
    "#,
    )
    .expect("valid Tera template");
}

#[launch]
fn rocket() -> _ {
    let figment = rocket::Config::figment().merge(("template_dir", TEMPLATE_DIR));

    rocket::custom(figment)
        .mount("/", routes![index, hello, about])
        .register("/", catchers![not_found])
        .attach(Template::custom(
            |tera| tera.register_filter("shout", shout),
            add_about_page,
        ))
}
