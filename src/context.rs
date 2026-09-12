use std::error::Error;
use std::path::{Path, PathBuf};

use crate::engine;

use normpath::PathExt;
use tera::Tera;

pub(crate) type Callback =
    Box<dyn Fn(&mut Tera) -> Result<(), Box<dyn Error>> + Send + Sync + 'static>;

pub(crate) struct Context {
    /// The root of the template directory.
    pub root: PathBuf,
    /// The initialized templating engine.
    pub tera: Tera,
}

pub(crate) use self::manager::ContextManager;

impl Context {
    /// Load all of the templates at `root`, initialize them using the relevant
    /// template engine, and store all of the initialized state in a `Context`
    /// structure, which is returned if all goes well.
    pub fn initialize(root: &Path, callback: &Callback) -> Option<Context> {
        let root = match root.normalize() {
            Ok(root) => root.into_path_buf(),
            Err(e) => {
                error!("Invalid template directory '{}': {}.", root.display(), e);
                return None;
            }
        };

        let mut files: Vec<(PathBuf, String)> = Vec::new();
        for entry in walkdir::WalkDir::new(&root).follow_links(true) {
            let entry = match entry {
                Ok(entry) if entry.file_type().is_file() => entry,
                Ok(_) | Err(_) => continue,
            };

            let name = template_name(&root, entry.path());
            files.push((entry.into_path(), name));
        }

        let mut tera = engine::init();
        if let Err(reason) = callback(&mut tera) {
            error_!("Template customization callback failed.");
            error_!("{}", reason);
            engine::log_error(&*reason);
            return None;
        }

        engine::load(&mut tera, &files)?;

        Some(Context { root, tera })
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

    use super::{Callback, Context};

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
        /// reinitialized from disk and the user's customization callback is run
        /// again.
        pub fn reload_if_needed(&self, callback: &Callback) {
            let templates_changes = self
                .watcher
                .as_ref()
                .map(|(_, rx)| rx.lock().expect("fsevents lock").try_iter().count() > 0);

            if let Some(true) = templates_changes {
                debug!("template change detected: reloading templates");
                let root = self.context().root.clone();
                if let Some(new_ctxt) = Context::initialize(&root, callback) {
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

/// Returns the name that identifies the template at `path`: its path relative
/// to `root`.
fn template_name(root: &Path, path: &Path) -> String {
    let rel_path = path.strip_prefix(root).unwrap();
    let mut name = rel_path.to_string_lossy().into_owned();

    // Ensure template name consistency on Windows systems
    if cfg!(windows) {
        name = name.replace('\\', "/");
    }

    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_path_index_html() {
        for root in &["/", "/a/b/c/", "/a/b/c/d/", "/a/"] {
            let path = Path::new(root).join("index.html");
            let name = template_name(Path::new(root), &path);

            assert_eq!(name, "index.html");
        }
    }

    #[test]
    fn template_path_subdir_index_html() {
        for root in &["/", "/a/b/c/", "/a/b/c/d/", "/a/"] {
            for sub in &["a/", "a/b/", "a/b/c/", "a/b/c/d/"] {
                let path = Path::new(root).join(sub).join("index.html");
                let name = template_name(Path::new(root), &path);

                let expected_name = format!("{sub}index.html");
                assert_eq!(name, expected_name.as_str());
            }
        }
    }
}
