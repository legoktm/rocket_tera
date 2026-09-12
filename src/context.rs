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

        let mut tera = engine::init();
        if let Err(reason) = callback(&mut tera) {
            error_!("Template customization callback failed.");
            error_!("{}", reason);
            engine::log_error(&*reason);
            return None;
        }

        engine::load(&mut tera, &root)?;

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

    use super::Context;
    use crate::engine;

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
        /// reloaded from disk.
        pub fn reload_if_needed(&self) {
            // Access events don't change templates, and reloading generates
            // them by reading every template, so ignore them to avoid reloading
            // on every request. Since notify v7, inotify always reports opens.
            // TODO: use `Config::with_event_kinds(EventKindMask::CORE)` once
            // notify v9 is released, so these events aren't watched at all.
            let templates_changes = self.watcher.as_ref().map(|(_, rx)| {
                rx.lock()
                    .expect("fsevents lock")
                    .try_iter()
                    .filter(|event| !event.as_ref().is_ok_and(|e| e.kind.is_access()))
                    .count()
                    > 0
            });

            if let Some(true) = templates_changes {
                debug!("template change detected: reloading templates");
                if engine::reload(&mut self.context_mut().tera).is_none() {
                    warn!(
                        "error while reloading template\n\
                        existing templates will remain active."
                    )
                };
            }
        }
    }
}
