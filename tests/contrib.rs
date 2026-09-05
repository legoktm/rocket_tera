//! Each `contrib-*` feature should register its filters, functions, and tests
//! without the application asking for them.

#![cfg(any(
    feature = "contrib-base64",
    feature = "contrib-date",
    feature = "contrib-filesize_format",
    feature = "contrib-format",
    feature = "contrib-json",
    feature = "contrib-rand",
    feature = "contrib-regex",
    feature = "contrib-slug",
    feature = "contrib-urlencode",
))]

use std::path::{Path, PathBuf};

use rocket::config::Config;
use rocket::local::blocking::Client;
use rocket::serde::Serialize;
use rocket_tera::{Template, context};

fn template_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("templates")
}

/// Renders `body` as a template, relying only on what the enabled `contrib-*`
/// features registered: no callback of our own registers anything.
#[track_caller]
fn render<C: Serialize>(body: &str, context: C) -> String {
    let body = body.to_string();
    let rocket = rocket::custom(Config::figment().merge(("template_dir", template_root()))).attach(
        Template::custom(
            |_| {},
            move |tera| {
                tera.add_raw_template("contrib.txt", &body)
                    .expect("valid Tera template");
            },
        ),
    );

    let client = Client::debug(rocket).expect("launch succeeds");
    Template::show(client.rocket(), "contrib.txt", context).expect("rendered")
}

#[test]
#[cfg(feature = "contrib-base64")]
fn test_base64() {
    assert_eq!(render(r#"{{ "hi" | b64_encode }}"#, context! {}), "aGk=");
    assert_eq!(render(r#"{{ "aGk=" | b64_decode }}"#, context! {}), "hi");
}

#[test]
#[cfg(feature = "contrib-date")]
fn test_date() {
    let ctx = context! { stamp: "2026-09-04T12:00:00Z" };
    assert_eq!(render("{{ stamp | date }}", &ctx), "2026-09-04");
    assert_eq!(
        render(r#"{{ stamp | date(format="%B %d, %Y") }}"#, &ctx),
        "September 04, 2026"
    );
    assert!(!render("{{ now() }}", context! {}).is_empty());
    assert_eq!(
        render(
            r#"{% if stamp is before(other="2030-01-01") %}yes{% endif %}"#,
            &ctx
        ),
        "yes"
    );
    assert_eq!(
        render(
            r#"{% if stamp is after(other="2020-01-01") %}yes{% endif %}"#,
            &ctx
        ),
        "yes"
    );
}

#[test]
#[cfg(feature = "contrib-filesize_format")]
fn test_filesize_format() {
    assert_eq!(render("{{ 1024 | filesize_format }}", context! {}), "1 KiB");
}

#[test]
#[cfg(feature = "contrib-format")]
fn test_format() {
    assert_eq!(
        render(r#"{{ 3.14159 | format(spec=".2") }}"#, context! {}),
        "3.14"
    );
}

#[test]
#[cfg(feature = "contrib-json")]
fn test_json() {
    assert_eq!(
        render("{{ items | json_encode }}", context! { items: [1, 2] }),
        "[1,2]"
    );
}

#[test]
#[cfg(feature = "contrib-rand")]
fn test_rand() {
    // `end` is exclusive, so this range has only one possible answer.
    assert_eq!(render("{{ get_random(start=1, end=2) }}", context! {}), "1");
    assert_eq!(
        render(
            "{{ items | shuffle | length }}",
            context! { items: [1, 2, 3] }
        ),
        "3"
    );
}

#[test]
#[cfg(feature = "contrib-regex")]
fn test_regex() {
    assert_eq!(
        render("{{ value | striptags }}", context! { value: "<b>hi</b>" }),
        "hi"
    );
    assert_eq!(
        render(
            "{{ value | spaceless }}",
            context! { value: "<p> <a></a> </p>" }
        ),
        "<p><a></a></p>"
    );
    assert_eq!(
        render(
            r#"{{ value | regex_replace(pattern="[0-9]+", rep="") }}"#,
            context! { value: "a1b2" }
        ),
        "ab"
    );
    assert_eq!(
        render(
            r#"{% if value is matching(pat="^hi") %}yes{% endif %}"#,
            context! { value: "hi there" }
        ),
        "yes"
    );
}

#[test]
#[cfg(feature = "contrib-slug")]
fn test_slug() {
    assert_eq!(
        render("{{ value | slug }}", context! { value: "Hello World" }),
        "hello-world"
    );
}

#[test]
#[cfg(feature = "contrib-urlencode")]
fn test_urlencode() {
    let ctx = context! { value: "a b/c" };
    assert_eq!(render("{{ value | urlencode }}", &ctx), "a%20b/c");
    assert_eq!(render("{{ value | urlencode_strict }}", &ctx), "a%20b%2Fc");
}

/// A `contrib-*` name is registered before the `register` callback runs, so an
/// application can still replace it.
#[test]
#[cfg(feature = "contrib-slug")]
fn test_contrib_name_can_be_overridden() {
    use rocket_tera::tera::{Kwargs, State};

    fn slug(_: &str, _: Kwargs, _: &State) -> String {
        "overridden".to_string()
    }

    let rocket = rocket::custom(Config::figment().merge(("template_dir", template_root()))).attach(
        Template::custom(
            |tera| tera.register_filter("slug", slug),
            |tera| {
                tera.add_raw_template("override.txt", "{{ value | slug }}")
                    .expect("valid Tera template");
            },
        ),
    );

    let client = Client::debug(rocket).expect("launch succeeds");
    assert_eq!(
        Template::show(
            client.rocket(),
            "override.txt",
            context! { value: "Hello World" }
        ),
        Some("overridden".into())
    );
}
