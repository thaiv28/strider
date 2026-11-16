mod app;
use strider_store::Store;
#[derive(Debug, Default, PartialEq, Eq)]
struct WorkspaceOptions {
    profile: String,
    reset: bool,
    server: String,
}
impl WorkspaceOptions {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            profile: "dev".into(),
            reset: false,
            server: "https://strider.thaiv.dev".into(),
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--server" => options.server = args.next().ok_or("--server requires a URL")?,
                "--reset" => options.reset = true,
                "--profile" => {
                    options.profile = args
                        .next()
                        .filter(|value| !value.starts_with('-'))
                        .ok_or("--profile requires a name")?;
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        Ok(options)
    }
}
fn open_store() -> Result<(Store, std::path::PathBuf, String), Box<dyn std::error::Error>> {
    let options = WorkspaceOptions::parse(std::env::args().skip(1))?;
    let path = Store::path_for_profile(&options.profile)?;
    if options.reset && path.exists() {
        std::fs::remove_file(&path)?;
    }
    let store = Store::open(&path)?;
    strider_sync::Client::new(&options.server, None)?;
    Ok((store, path, options.server))
}
fn main() {
    let (store, path, server) = match open_store() {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Could not open the Strider workspace: {error}");
            return;
        }
    };
    app::launch(store, path, server);
}
