#[macro_use]
extern crate rocket;

mod tera;

#[cfg(test)]
mod tests;

use rocket::response::content::RawHtml;
use rocket_tera::Template;

/// Templates live next to this example, not in the current working directory,
/// so that `cargo run --example templating` works from anywhere in the repo.
const TEMPLATE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/templating/templates");

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml(r#"See <a href="tera">Tera</a>."#)
}

#[launch]
fn rocket() -> _ {
    let figment = rocket::Config::figment().merge(("template_dir", TEMPLATE_DIR));

    rocket::custom(figment)
        .mount("/", routes![index])
        .mount("/tera", routes![tera::index, tera::hello, tera::about])
        .register("/tera", catchers![tera::not_found])
        .attach(Template::custom(|engines| {
            tera::customize(&mut engines.tera);
        }))
}
