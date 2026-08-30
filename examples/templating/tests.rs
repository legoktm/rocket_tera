use super::rocket;

use rocket::http::{Method::*, RawStr, Status};
use rocket::local::blocking::Client;
use rocket_tera::{Template, context};

#[test]
fn test_root_redirects() {
    let client = Client::tracked(rocket()).unwrap();
    for method in &[Get, Head] {
        let response = client.req(*method, "/").dispatch();
        assert_eq!(response.status(), Status::SeeOther);
        assert!(response.body().is_none());

        let location = response.headers().get_one("Location").unwrap();
        assert_eq!(location, "/hello/Your%20Name");
    }
}

#[test]
fn test_root_other_methods_are_caught() {
    let client = Client::tracked(rocket()).unwrap();
    for method in &[Post, Put, Delete, Options, Trace, Connect, Patch] {
        let context = context! { uri: "/" };
        let expected = Template::show(client.rocket(), "error/404", &context);

        let response = client.req(*method, "/").dispatch();
        assert_eq!(response.status(), Status::NotFound);
        assert_eq!(response.into_string(), expected);
    }
}

#[test]
fn test_hello() {
    let client = Client::tracked(rocket()).unwrap();
    let response = client.get("/hello/Jack%20Daniels").dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert!(response.into_string().unwrap().contains("Hi Jack Daniels!"));
}

#[test]
fn test_404() {
    let client = Client::tracked(rocket()).unwrap();
    for path in &["/hello", "/foo/bar", "/404"] {
        let escaped = RawStr::new(path).html_escape().to_lowercase();

        let response = client.get(*path).dispatch();
        assert_eq!(response.status(), Status::NotFound);
        let response = response.into_string().unwrap().to_lowercase();

        assert! {
            response.contains(&format!("{} does not exist", path))
                || response.contains(&format!("{} does not exist", escaped))
        };
    }
}

#[test]
fn test_about() {
    let client = Client::tracked(rocket()).unwrap();
    let response = client.get("/about").dispatch();
    assert!(
        response
            .into_string()
            .unwrap()
            .contains("About - Here's another page!")
    );
}
