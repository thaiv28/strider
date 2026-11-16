mod app;
use strider_store::{Store, seed};
#[derive(Debug, Default, PartialEq, Eq)]
struct WorkspaceOptions {
    profile: String,
    reset: bool,
}
impl WorkspaceOptions {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            profile: "dev".into(),
            reset: false,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
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
fn open_store() -> Result<Store, Box<dyn std::error::Error>> {
    let options = WorkspaceOptions::parse(std::env::args().skip(1))?;
    let path = Store::path_for_profile(&options.profile)?;
    if options.reset && path.exists() {
        std::fs::remove_file(&path)?;
    }
    let needs_seed = !path.exists();
    let mut store = Store::open(&path)?;
    if needs_seed {
        seed(&mut store, 2000)?;
    }
    Ok(store)
}
fn main() {
    let store = match open_store() {
        Ok(store) => store,
        Err(error) => {
            eprintln!("Could not open the Strider workspace: {error}");
            return;
        }
    };
    app::launch(store);
}
