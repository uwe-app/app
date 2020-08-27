use cache::{self, CacheComponent};
use config::server::{ServerConfig, LaunchConfig};
use crate::Error;

static DOCS_DIR: &str = "docs";

pub async fn open() -> Result<(), Error> {
    let prefs = preference::load()?;
    let docs_prefs = prefs.docs.as_ref().unwrap();

    // Served from a sub-directory
    let mut target = cache::get_docs_dir()?;

    if !target.exists() {
        cache::update(&prefs, vec![CacheComponent::Documentation])?;
    }

    target.push(DOCS_DIR);

    let opts = ServerConfig {
        target,
        host: docs_prefs.host.clone(),
        port: docs_prefs.port.clone(),
        tls: None,
        watch: None,
        endpoint: utils::generate_id(16),
        redirects: None,
        log: true,
        temporary_redirect: true,
        disable_cache: true,
        redirect_insecure: true,
    };

    let launch = LaunchConfig { open: true };
    Ok(server::bind(opts, launch, None).await?)
}
