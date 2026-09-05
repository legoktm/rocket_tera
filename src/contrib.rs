//! Registration of the [`tera-contrib`] filters, functions, and tests that the
//! enabled `contrib-*` features ask for.
//!
//! Tera 2 dropped the builtins that needed third-party dependencies; they live
//! in `tera-contrib` now. Rather than making every user write out the
//! [`Tera::register_filter()`] calls, each `contrib-{name}` feature enables the
//! `{name}` feature of `tera-contrib` and registers what it provides under the
//! names its documentation uses.
//!
//! [`tera-contrib`]: https://docs.rs/tera-contrib
//! [`Tera::register_filter()`]: tera::Tera::register_filter()

use tera::Tera;

/// Registers everything provided by the enabled `contrib-*` features.
///
/// This runs before the `register` callback given to
/// [`Template::custom()`](crate::Template::custom()), so a name registered
/// here can be replaced by registering it again there.
pub(crate) fn register(#[allow(unused_variables)] tera: &mut Tera) {
    #[cfg(feature = "contrib-base64")]
    {
        tera.register_filter("b64_encode", tera_contrib::base64::b64_encode);
        tera.register_filter("b64_decode", tera_contrib::base64::b64_decode);
    }

    #[cfg(feature = "contrib-date")]
    {
        tera.register_function("now", tera_contrib::dates::now);
        tera.register_filter("date", tera_contrib::dates::date);
        tera.register_test("before", tera_contrib::dates::is_before);
        tera.register_test("after", tera_contrib::dates::is_after);
    }

    #[cfg(feature = "contrib-filesize_format")]
    tera.register_filter(
        "filesize_format",
        tera_contrib::filesize_format::filesize_format,
    );

    #[cfg(feature = "contrib-format")]
    tera.register_filter("format", tera_contrib::format::format);

    #[cfg(feature = "contrib-json")]
    tera.register_filter("json_encode", tera_contrib::json::json_encode);

    #[cfg(feature = "contrib-rand")]
    {
        tera.register_function("get_random", tera_contrib::rand::get_random);
        tera.register_filter("shuffle", tera_contrib::rand::shuffle);
    }

    #[cfg(feature = "contrib-regex")]
    {
        tera.register_filter("striptags", tera_contrib::regex::striptags);
        tera.register_filter("spaceless", tera_contrib::regex::spaceless);
        tera.register_filter(
            "regex_replace",
            tera_contrib::regex::RegexReplace::default(),
        );
        tera.register_test("matching", tera_contrib::regex::Matching::default());
    }

    #[cfg(feature = "contrib-slug")]
    tera.register_filter("slug", tera_contrib::slug::slug);

    #[cfg(feature = "contrib-urlencode")]
    {
        tera.register_filter("urlencode", tera_contrib::urlencode::urlencode);
        tera.register_filter(
            "urlencode_strict",
            tera_contrib::urlencode::urlencode_strict,
        );
    }
}
