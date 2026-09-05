use rocket::fairing::{self, Fairing, Info, Kind};
use rocket::figment::{Source, value::magic::RelativePathBuf};
use rocket::{Build, Orbit, Rocket};

use crate::context::{Callbacks, Context, ContextManager};
use crate::template::DEFAULT_TEMPLATE_DIR;

/// The TemplateFairing initializes the template system on attach, running the
/// `register` callback before templates are loaded and the `finalize`
/// callback after. In debug mode, the fairing checks for modifications to
/// templates before every request and reloads them if necessary.
pub(crate) struct TemplateFairing {
    /// The user-provided customization callbacks, allowing the use of
    /// functionality specific to the template engine. In debug mode, these
    /// callbacks might be run multiple times as templates are reloaded.
    pub(crate) callbacks: Callbacks,
}

#[rocket::async_trait]
impl Fairing for TemplateFairing {
    fn info(&self) -> Info {
        let kind = Kind::Ignite | Kind::Liftoff;
        #[cfg(debug_assertions)]
        let kind = kind | Kind::Request;

        Info {
            kind,
            name: "Templating",
        }
    }

    /// Initializes the template context. Templates will be searched for in the
    /// `template_dir` config variable or the default ([DEFAULT_TEMPLATE_DIR]).
    /// The user's callbacks, if any were supplied, are called to customize the
    /// template engine. In debug mode, the `ContextManager::new` method
    /// initializes a directory watcher for auto-reloading of templates.
    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        let configured_dir = rocket
            .figment()
            .extract_inner::<RelativePathBuf>("template_dir")
            .map(|path| path.relative());

        let path = match configured_dir {
            Ok(dir) => dir,
            Err(e) if e.missing() => DEFAULT_TEMPLATE_DIR.into(),
            Err(e) => {
                error_!("Invalid `template_dir` configuration: {}", e);
                return Err(rocket);
            }
        };

        if let Some(ctxt) = Context::initialize(&path, &self.callbacks) {
            Ok(rocket.manage(ContextManager::new(ctxt)))
        } else {
            error_!("Template initialization failed. Aborting launch.");
            Err(rocket)
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let cm = rocket
            .state::<ContextManager>()
            .expect("Template ContextManager registered in on_ignite");

        use rocket::{log::PaintExt, yansi::Paint};

        info!("{}{}:", "📐 ".emoji(), "Templating".magenta());
        info_!("directory: {}", Source::from(&*cm.context().root).primary());
    }

    #[cfg(debug_assertions)]
    async fn on_request(&self, req: &mut rocket::Request<'_>, _data: &mut rocket::Data<'_>) {
        let cm = req
            .rocket()
            .state::<ContextManager>()
            .expect("Template ContextManager registered in on_ignite");

        cm.reload_if_needed(&self.callbacks);
    }
}
