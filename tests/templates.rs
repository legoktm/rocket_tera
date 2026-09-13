#[macro_use]
extern crate rocket;

use std::path::{Path, PathBuf};

use rocket::config::Config;
use rocket::figment::value::Value;
use rocket::serde::{Deserialize, Serialize};
use rocket::{Build, Rocket};
use rocket_tera::tera::{Kwargs, State, Tera};
use rocket_tera::{Metadata, Template, context};

#[get("/<name>")]
fn template_check(md: Metadata<'_>, name: &str) -> Option<()> {
    md.contains_template(name).then_some(())
}

#[get("/is_reloading")]
fn is_reloading(md: Metadata<'_>) -> Option<()> {
    if md.reloading() { Some(()) } else { None }
}

fn template_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("templates")
}

fn rocket() -> Rocket<Build> {
    rocket::custom(Config::figment().merge(("template_dir", template_root())))
        .attach(Template::fairing())
        .mount("/", routes![template_check, is_reloading])
}

/// Asserts that launching `rocket` fails because the "Templating" fairing did.
#[track_caller]
fn assert_templating_fairing_failed(rocket: Rocket<Build>) {
    use rocket::{error::ErrorKind::FailedFairings, local::blocking::Client};

    let error = Client::debug(rocket).expect_err("client failure");
    match error.kind() {
        FailedFairings(failures) => assert_eq!(failures[0].name, "Templating"),
        _ => panic!("Wrong kind of launch error"),
    }
}

#[test]
fn test_callback_error() {
    assert_templating_fairing_failed(
        rocket::build().attach(Template::try_custom(|_| Err("error with Tera!".into()))),
    );
}

/// A custom filter, registered by the callback ordering tests below.
fn shout(value: &str, _: Kwargs, _: &State) -> String {
    value.to_uppercase()
}

/// Registers a template that extends `base.txt`, which is only loaded from
/// disk, so this can only succeed once template loading has happened.
fn add_extending_template(tera: &mut Tera) -> Result<(), Box<dyn std::error::Error>> {
    tera.add_raw_template(
        "extends.txt",
        r#"{% extends "base.txt" %}{% block content %}{{ value }}{% endblock content %}"#,
    )?;

    Ok(())
}

/// Builds a Rocket serving [`order_templates`], which contains a template using
/// the `shout` filter and a base template meant to be extended.
fn order_rocket() -> Rocket<Build> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("order_templates");

    rocket::custom(Config::figment().merge(("template_dir", root)))
}

#[test]
fn test_callback_ordering() {
    use rocket::local::blocking::Client;

    // The callback runs first, so its filter is known by the time the template
    // using it is loaded.
    let rocket = order_rocket().attach(Template::custom(|tera| {
        tera.register_filter("shout", shout);
    }));

    let client = Client::debug(rocket).expect("launch succeeds");
    let ctx = context! { value: "hi" };
    assert_eq!(
        Template::show(client.rocket(), "filtered.txt", &ctx),
        Some("HI".into())
    );
}

#[test]
fn test_extending_too_early() {
    // Conversely, extending `base.txt` in the callback is too early: nothing
    // has been loaded from disk yet.
    assert_templating_fairing_failed(
        order_rocket().attach(Template::try_custom(add_extending_template)),
    );
}

#[get("/")]
fn sentinel_return_template() -> Template {
    Template::render("foo", ())
}

#[get("/")]
fn sentinel_return_opt_template() -> Option<Template> {
    Some(Template::render("foo", ()))
}

#[derive(rocket::Responder)]
struct SentinelMyThing<T>(T);

#[get("/")]
fn sentinel_return_custom_template() -> SentinelMyThing<Template> {
    SentinelMyThing(Template::render("foo", ()))
}

#[derive(rocket::Responder)]
struct SentinelMyOkayThing<T>(Option<T>);

impl<T> rocket::Sentinel for SentinelMyOkayThing<T> {
    fn abort(_: &Rocket<rocket::Ignite>) -> bool {
        false
    }
}

#[get("/")]
fn always_ok_sentinel() -> SentinelMyOkayThing<Template> {
    SentinelMyOkayThing(None)
}

#[test]
fn test_sentinel() {
    use rocket::{error::ErrorKind::SentinelAborts, local::blocking::Client};

    let err = Client::debug_with(routes![is_reloading]).unwrap_err();
    assert!(matches!(err.kind(), SentinelAborts(vec) if vec.len() == 1));

    let err = Client::debug_with(routes![is_reloading, template_check]).unwrap_err();
    assert!(matches!(err.kind(), SentinelAborts(vec) if vec.len() == 2));

    let err = Client::debug_with(routes![sentinel_return_template]).unwrap_err();
    assert!(matches!(err.kind(), SentinelAborts(vec) if vec.len() == 1));

    let err = Client::debug_with(routes![sentinel_return_opt_template]).unwrap_err();
    assert!(matches!(err.kind(), SentinelAborts(vec) if vec.len() == 1));

    let err = Client::debug_with(routes![sentinel_return_custom_template]).unwrap_err();
    assert!(matches!(err.kind(), SentinelAborts(vec) if vec.len() == 1));

    Client::debug_with(routes![always_ok_sentinel]).expect("no sentinel abort");
}

#[test]
fn test_context_macro() {
    macro_rules! assert_same_object {
        ($ctx:expr, $obj:expr $(,)?) => {{
            let ser_ctx = Value::serialize(&$ctx).unwrap();
            let deser_ctx = ser_ctx.deserialize().unwrap();
            assert_eq!($obj, deser_ctx);
        }};
    }

    {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(crate = "rocket::serde")]
        struct Empty {}

        assert_same_object!(context! {}, Empty {});
    }

    {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(crate = "rocket::serde")]
        struct Object {
            a: u32,
            b: String,
        }

        let a = 93;
        let b = "Hello".to_string();

        fn make_context() -> impl Serialize {
            let b = "Hello".to_string();

            context! { a: 93, b: b }
        }

        assert_same_object!(make_context(), Object { a, b },);
    }

    {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(crate = "rocket::serde")]
        struct Outer {
            s: String,
            inner: Inner,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(crate = "rocket::serde")]
        struct Inner {
            center: Center,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(crate = "rocket::serde")]
        struct Center {
            value_a: bool,
            value_b: u8,
        }

        let a = true;
        let value_b = 123;
        let outer_string = String::from("abc 123");

        assert_same_object!(
            context! {
                s: &outer_string,
                inner: context! {
                    center: context! {
                        value_a: a,
                        value_b,
                    },
                },
            },
            Outer {
                s: outer_string,
                inner: Inner {
                    center: Center {
                        value_a: a,
                        value_b,
                    },
                },
            },
        );
    }

    {
        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(crate = "rocket::serde")]
        struct Object {
            a: String,
        }

        let owned = String::from("foo");
        let ctx = context! { a: &owned };
        assert_same_object!(ctx, Object { a: "foo".into() });
        // The explicit drops are the point: the context must be droppable
        // before the value it borrows.
        #[allow(clippy::drop_non_drop)]
        {
            drop(ctx);
            drop(owned);
        }
    }
}

mod tera_tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rocket::http::{ContentType, Status};
    use rocket::request::FromRequest;
    use std::collections::HashMap;

    const UNESCAPED_EXPECTED: &str = "\nh_start\ntitle: _test_\nh_end\n\n\n<script />\n\nfoot";
    const ESCAPED_EXPECTED: &str = "\nh_start\ntitle: _test_\nh_end\n\n\n&lt;script /&gt;\n\nfoot";

    /// Converts CRLF line endings to LF, since Git on Windows may check the
    /// template fixtures out with CRLF.
    fn normalize_line_endings(content: String) -> String {
        content.replace("\r\n", "\n")
    }

    #[async_test]
    async fn test_tera_templates() {
        use rocket::local::asynchronous::Client;

        let client = Client::debug(rocket()).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();

        let mut map = HashMap::new();
        map.insert("title", "_test_");
        map.insert("content", "<script />");
        let show = |name| Template::show(client.rocket(), name, &map).map(normalize_line_endings);
        let md_render = |name| {
            metadata
                .render(name, &map)
                .map(|(ct, s)| (ct, normalize_line_endings(s)))
        };

        // Test with a txt file, which shouldn't escape.
        let template = show("txt_test.txt");
        let md_rendered = md_render("txt_test.txt");
        assert_eq!(template, Some(UNESCAPED_EXPECTED.into()));
        assert_eq!(
            md_rendered,
            Some((ContentType::Text, UNESCAPED_EXPECTED.into()))
        );

        // Now with an HTML file, which should escape.
        let template = show("html_test.html");
        let md_rendered = md_render("html_test.html");
        assert_eq!(template, Some(ESCAPED_EXPECTED.into()));
        assert_eq!(
            md_rendered,
            Some((ContentType::HTML, ESCAPED_EXPECTED.into()))
        );
    }

    #[async_test]
    async fn test_globby_paths() {
        use rocket::local::asynchronous::Client;

        let client = Client::debug(rocket()).await.unwrap();
        let req = client.get("/");
        let metadata = Metadata::from_request(&req).await.unwrap();
        assert!(metadata.contains_template("[test]/html_test.html"));
    }

    // u128 is not supported. enable when it is.
    // #[test]
    // fn test_tera_u128() {
    //     const EXPECTED: &'static str
    //         = "\nh_start\ntitle: 123\nh_end\n\n\n1208925819614629174706176\n\nfoot\n";
    //
    //     let client = Client::debug(rocket()).unwrap();
    //     let mut map = HashMap::new();
    //     map.insert("title", 123);
    //     map.insert("number", 1u128 << 80);
    //
    //     let template = Template::show(client.rocket(), "txt_test.txt", &map);
    //     assert_eq!(template, Some(EXPECTED.into()));
    // }

    #[test]
    fn test_template_metadata_with_tera() {
        use rocket::local::blocking::Client;

        let client = Client::debug(rocket()).unwrap();

        let response = client.get("/txt_test.txt").dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/html_test.html").dispatch();
        assert_eq!(response.status(), Status::Ok);

        let response = client.get("/not_existing").dispatch();
        assert_eq!(response.status(), Status::NotFound);
    }

    #[cfg(debug_assertions)]
    fn write_file(path: &Path, text: &str) {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path).expect("open file");
        file.write_all(text.as_bytes()).expect("write file");
        file.sync_all().expect("sync file");
    }

    /// Creates an empty template directory named `name`, so that a test can
    /// modify its templates without affecting tests running in parallel.
    #[cfg(debug_assertions)]
    fn scratch_template_dir(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create template dir");
        dir
    }

    /// Returns a client serving the templates in `dir`, or `None` if template
    /// reloading is unavailable.
    #[cfg(debug_assertions)]
    fn reloading_client(dir: &Path) -> Option<rocket::local::blocking::Client> {
        let rocket = rocket::custom(Config::figment().merge(("template_dir", dir)))
            .attach(Template::fairing())
            .mount("/", routes![is_reloading]);

        let client = rocket::local::blocking::Client::debug(rocket).unwrap();
        let status = client.get("/is_reloading").dispatch().status();
        (status == Status::Ok).then_some(client)
    }

    /// Dispatches requests, each of which triggers a template reload if
    /// needed, until `name` renders as `expected`. Gives up after 1.5s.
    #[cfg(debug_assertions)]
    fn wait_for_render(
        client: &rocket::local::blocking::Client,
        name: &'static str,
        expected: &str,
    ) -> bool {
        for _ in 0..6 {
            client.get("/").dispatch();
            let rendered = Template::show(client.rocket(), name, context! {});
            if rendered.as_deref() == Some(expected) {
                return true;
            }

            std::thread::sleep(std::time::Duration::from_millis(250));
        }

        false
    }

    #[test]
    #[cfg(debug_assertions)]
    fn test_template_reload_new_file() {
        let dir = scratch_template_dir("reload_new_file");
        write_file(&dir.join("existing.txt"), "existing");
        let Some(client) = reloading_client(&dir) else {
            return;
        };

        assert_eq!(
            Template::show(client.rocket(), "new.txt", context! {}),
            None
        );

        write_file(&dir.join("new.txt"), "new");
        assert!(
            wait_for_render(&client, "new.txt", "new"),
            "failed to load new template in 1.5s"
        );
        assert_eq!(
            Template::show(client.rocket(), "existing.txt", context! {}),
            Some("existing".into())
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    fn test_template_reload_broken_edit() {
        let dir = scratch_template_dir("reload_broken_edit");
        let page_path = dir.join("page.txt");
        write_file(&page_path, "initial");
        let Some(client) = reloading_client(&dir) else {
            return;
        };

        // Break an existing template and add a valid new one. The reload fails
        // as a whole, so neither change is picked up.
        write_file(&page_path, "{{ broken");
        write_file(&dir.join("new.txt"), "new");
        assert!(
            !wait_for_render(&client, "new.txt", "new"),
            "loaded new template despite a broken template"
        );
        assert_eq!(
            Template::show(client.rocket(), "page.txt", context! {}),
            Some("initial".into())
        );

        // Once the template is fixed, both changes are picked up.
        write_file(&page_path, "fixed");
        assert!(
            wait_for_render(&client, "page.txt", "fixed"),
            "failed to reload fixed template in 1.5s"
        );
        assert_eq!(
            Template::show(client.rocket(), "new.txt", context! {}),
            Some("new".into())
        );
    }

    /// Reloading reads every template, which the watcher may report as access
    /// events (inotify does since notify v7). Those must not trigger another
    /// reload, or templates are reloaded on every request.
    #[test]
    #[cfg(all(debug_assertions, unix))]
    fn test_template_reload_ignores_own_reads() {
        let dir = scratch_template_dir("reload_ignores_own_reads");
        let outside = scratch_template_dir("reload_ignores_own_reads_outside");

        // Changes to a symlink's target outside the template directory aren't
        // reported by the watcher, but are picked up by any reload. This makes
        // a reload visible even when nothing in the template directory changed.
        let target_path = outside.join("target.txt");
        write_file(&target_path, "initial");
        std::os::unix::fs::symlink(&target_path, dir.join("link.txt")).expect("symlink");

        let trigger_path = dir.join("trigger.txt");
        write_file(&trigger_path, "initial");
        let Some(client) = reloading_client(&dir) else {
            return;
        };

        // Cause one legitimate reload, which reads every template.
        write_file(&trigger_path, "changed");
        assert!(
            wait_for_render(&client, "trigger.txt", "changed"),
            "failed to reload modified template in 1.5s"
        );

        // Nothing in the template directory changes from here on, so no further
        // requests should reload templates.
        write_file(&target_path, "changed");
        assert!(
            !wait_for_render(&client, "link.txt", "changed"),
            "templates were reloaded without any changes"
        );
    }

    /// Dot-files, such as Vim swap files, aren't templates. They must not be
    /// loaded, or one that isn't valid UTF-8 breaks loading and reloading.
    #[test]
    #[cfg(debug_assertions)]
    fn test_template_reload_ignores_dotfiles() {
        let dir = scratch_template_dir("reload_ignores_dotfiles");
        let page_path = dir.join("page.txt");
        write_file(&page_path, "initial");
        write_file(&dir.join(".gitkeep"), "");
        std::fs::write(dir.join(".page.txt.swp"), [0xb0, 0xff, 0xfe]).expect("write swap file");
        let Some(client) = reloading_client(&dir) else {
            return;
        };

        assert_eq!(
            Template::show(client.rocket(), "page.txt", context! {}),
            Some("initial".into())
        );
        assert_eq!(
            Template::show(client.rocket(), ".gitkeep", context! {}),
            None
        );

        write_file(&page_path, "changed");
        assert!(
            wait_for_render(&client, "page.txt", "changed"),
            "failed to reload modified template in 1.5s"
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    fn test_template_reload() {
        let dir = scratch_template_dir("reload");
        let reload_path = dir.join("reload.txt");
        write_file(&reload_path, "initial");
        let Some(client) = reloading_client(&dir) else {
            return;
        };

        assert_eq!(
            Template::show(client.rocket(), "reload.txt", context! {}),
            Some("initial".into())
        );

        write_file(&reload_path, "reload");
        assert!(
            wait_for_render(&client, "reload.txt", "reload"),
            "failed to reload modified template in 1.5s"
        );
    }
}
