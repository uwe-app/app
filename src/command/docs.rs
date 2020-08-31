use cache::{self, CacheComponent};
use config::server::{ServerConfig, LaunchConfig, HostConfig};
use crate::Error;

static DOCS_DIR: &str = "docs";

pub async fn open() -> Result<(), Error> {
    let prefs = preference::load()?;
    let docs_prefs = prefs.docs.as_ref().unwrap();

    // Served from a sub-directory
    let target = cache::get_docs_dir()?;

    if !target.exists() {
        cache::update(&prefs, vec![CacheComponent::Documentation])?;
    }

    let target = target.join(DOCS_DIR);

    //println!("Docs target is {:#?}", target);

    let tls = None;
    let host = HostConfig::new(target, docs_prefs.host.to_owned(), None, None);
    let opts = ServerConfig::new_host(host, docs_prefs.port.to_owned(), tls);
    let launch = LaunchConfig { open: true };

    // Convert to &'static reference
    let opts = server::configure(opts);
    let mut channels = Default::default();
    Ok(server::launch(opts, launch, &mut channels).await?)
}
