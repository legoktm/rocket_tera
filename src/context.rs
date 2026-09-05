use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::engine;
use crate::template::TemplateInfo;

use normpath::PathExt;
use rocket::http::ContentType;
use tera::Tera;

pub(crate) type Callback =
    Box<dyn Fn(&mut Tera) -> Result<(), Box<dyn Error>> + Send + Sync + 'static>;

/// The pair of user-provided callbacks run around template loading.
pub(crate) struct Callbacks {
    pub(crate) register: Callback,
    pub(crate) finalize: Callback,
}

pub(crate) struct Context {
    /// The root of the template directory.
    pub root: PathBuf,
    /// Mapping from template name to its information.
    pub templates: HashMap<String, TemplateInfo>,
    /// The initialized templating engine.
    pub tera: Tera,
}

pub(crate) use self::manager::ContextManager;

impl Context {
    /// Load all of the templates at `root`, initialize them using the relevant
    /// template engine, and store all of the initialized state in a `Context`
    /// structure, which is returned if all goes well.
    pub fn initialize(root: &Path, callbacks: &Callbacks) -> Option<Context> {
        let root = match root.normalize() {
            Ok(root) => root.into_path_buf(),
            Err(e) => {
                error!("Invalid template directory '{}': {}.", root.display(), e);
                return None;
            }
        };

        let mut templates: HashMap<String, TemplateInfo> = HashMap::new();
        for entry in walkdir::WalkDir::new(&root).follow_links(true) {
            let entry = match entry {
                Ok(entry) if entry.file_type().is_file() => entry,
                Ok(_) | Err(_) => continue,
            };

            let (template, data_type_str) = split_path(&root, entry.path());
            if let Some(info) = templates.get(&*template) {
                warn_!(
                    "Template name '{}' does not have a unique source.",
                    template
                );
                match info.path {
                    Some(ref path) => info_!("Existing path: {:?}", path),
                    None => info_!("Existing Content-Type: {}", info.data_type),
                }

                info_!("Additional path: {:?}", entry.path());
                warn_!("Keeping existing template '{}'.", template);

                continue;
            }

            let data_type = data_type_str
                .as_ref()
                .and_then(|ext| ContentType::from_extension(ext))
                .unwrap_or(ContentType::Text);

            templates.insert(
                template,
                TemplateInfo {
                    path: Some(entry.into_path()),
                    data_type,
                },
            );
        }

        // We load in 3 stages:
        // 1) user-specified registration of filters, functions, etc.
        // 2) loading templates from disk
        // 3) user-specified finalization, which could be loading other templates
        let mut tera = engine::init();
        run(&callbacks.register, &mut tera, "register")?;
        engine::load(&mut tera, &templates)?;
        run(&callbacks.finalize, &mut tera, "finalize")?;

        for name in tera.get_template_names() {
            if !templates.contains_key(name) {
                let data_type = Path::new(name)
                    .extension()
                    .and_then(|osstr| osstr.to_str())
                    .and_then(ContentType::from_extension)
                    .unwrap_or(ContentType::Text);

                let info = TemplateInfo {
                    path: None,
                    data_type,
                };
                templates.insert(name.to_string(), info);
            }
        }

        Some(Context {
            root,
            templates,
            tera,
        })
    }
}

/// Runs one user-provided `callback`, named `name` in any error message.
fn run(callback: &Callback, tera: &mut Tera, name: &str) -> Option<()> {
    match callback(tera) {
        Ok(()) => Some(()),
        Err(reason) => {
            error_!("Template `{}` callback failed.", name);
            error_!("{}", reason);
            engine::log_error(&*reason);
            None
        }
    }
}

#[cfg(not(debug_assertions))]
mod manager {
    use super::Context;
    use std::ops::Deref;

    /// Wraps a Context. With `cfg(debug_assertions)` active, this structure
    /// additionally provides a method to reload the context at runtime.
    pub(crate) struct ContextManager(Context);

    impl ContextManager {
        pub fn new(ctxt: Context) -> ContextManager {
            ContextManager(ctxt)
        }

        pub fn context<'a>(&'a self) -> impl Deref<Target = Context> + 'a {
            &self.0
        }

        pub fn is_reloading(&self) -> bool {
            false
        }
    }
}

#[cfg(debug_assertions)]
mod manager {
    use std::ops::{Deref, DerefMut};
    use std::sync::mpsc::{Receiver, channel};
    use std::sync::{Mutex, RwLock};

    use notify::{Error, Event, RecommendedWatcher, RecursiveMode, Watcher, recommended_watcher};

    use super::{Callbacks, Context};

    /// A filesystem watcher paired with the receive queue for its events.
    type Watched = (RecommendedWatcher, Mutex<Receiver<Result<Event, Error>>>);

    /// Wraps a Context. With `cfg(debug_assertions)` active, this structure
    /// additionally provides a method to reload the context at runtime.
    pub(crate) struct ContextManager {
        /// The current template context, inside an RwLock so it can be updated.
        context: RwLock<Context>,
        /// A filesystem watcher and the receive queue for its events.
        watcher: Option<Watched>,
    }

    impl ContextManager {
        pub fn new(ctxt: Context) -> ContextManager {
            let (tx, rx) = channel();
            let watcher = recommended_watcher(tx).and_then(|mut watcher| {
                watcher.watch(&ctxt.root.canonicalize()?, RecursiveMode::Recursive)?;
                Ok(watcher)
            });

            let watcher = match watcher {
                Ok(watcher) => Some((watcher, Mutex::new(rx))),
                Err(e) => {
                    warn!(
                        "live template reloading initialization failed: {e}\n\
                        live template reloading is unavailable"
                    );
                    None
                }
            };

            ContextManager {
                watcher,
                context: RwLock::new(ctxt),
            }
        }

        pub fn context(&self) -> impl Deref<Target = Context> + '_ {
            self.context.read().unwrap()
        }

        pub fn is_reloading(&self) -> bool {
            self.watcher.is_some()
        }

        fn context_mut(&self) -> impl DerefMut<Target = Context> + '_ {
            self.context.write().unwrap()
        }

        /// Checks whether any template files have changed on disk. If there
        /// have been changes since the last reload, all templates are
        /// reinitialized from disk and the user's customization callbacks are
        /// run again.
        pub fn reload_if_needed(&self, callbacks: &Callbacks) {
            let templates_changes = self
                .watcher
                .as_ref()
                .map(|(_, rx)| rx.lock().expect("fsevents lock").try_iter().count() > 0);

            if let Some(true) = templates_changes {
                debug!("template change detected: reloading templates");
                let root = self.context().root.clone();
                if let Some(new_ctxt) = Context::initialize(&root, callbacks) {
                    *self.context_mut() = new_ctxt;
                } else {
                    warn!(
                        "error while reloading template\n\
                        existing templates will remain active."
                    )
                };
            }
        }
    }
}

/// Splits a path into a name that may be used to identify the template, and the
/// template's data type, if any.
fn split_path(root: &Path, path: &Path) -> (String, Option<String>) {
    let rel_path = path.strip_prefix(root).unwrap().to_path_buf();
    let data_type = rel_path.extension();
    let mut name = rel_path.to_string_lossy().into_owned();

    // Ensure template name consistency on Windows systems
    if cfg!(windows) {
        name = name.replace('\\', "/");
    }

    (name, data_type.map(|d| d.to_string_lossy().into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_path_index_html() {
        for root in &["/", "/a/b/c/", "/a/b/c/d/", "/a/"] {
            let path = Path::new(root).join("index.html");
            let (name, data_type) = split_path(Path::new(root), &path);

            assert_eq!(name, "index.html");
            assert_eq!(data_type, Some("html".into()));
        }
    }

    #[test]
    fn template_path_subdir_index_html() {
        for root in &["/", "/a/b/c/", "/a/b/c/d/", "/a/"] {
            for sub in &["a/", "a/b/", "a/b/c/", "a/b/c/d/"] {
                let path = Path::new(root).join(sub).join("index.html");
                let (name, data_type) = split_path(Path::new(root), &path);

                let expected_name = format!("{sub}index.html");
                assert_eq!(name, expected_name.as_str());
                assert_eq!(data_type, Some("html".into()));
            }
        }
    }
}
