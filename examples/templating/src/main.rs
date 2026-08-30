#[macro_use]
extern crate rocket;

mod tera;

#[cfg(test)]
mod tests;

use rocket::response::content::RawHtml;
use rocket_tera::Template;

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml(r#"See <a href="tera">Tera</a>."#)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/tera", routes![tera::index, tera::hello, tera::about])
        .register("/tera", catchers![tera::not_found])
        .attach(Template::custom(|engines| {
            tera::customize(&mut engines.tera);
        }))
}
