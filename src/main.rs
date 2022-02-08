use weise::commands;

#[derive(Debug, clap::Parser)]
#[clap(name = "weise", about = "微博搜索")]
struct Config {
    #[clap(
        short = 'v',
        long = "--verbose",
        multiple_occurrences = true,
        parse(from_occurrences)
    )]
    pub log_verbose_count: u8,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Parser)]
enum Command {
    Crawl(commands::crawl::Config),
    Index(commands::index::Config),
    Search(commands::search::Config),
    Tombstone(commands::tombstone::Config),
    Settings(commands::settings::Config),
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let global_config: Config = clap::Parser::parse();
    init_simple_logs(global_config.log_verbose_count);

    match global_config.command {
        Command::Crawl(config) => commands::crawl::command(config).await?,
        Command::Index(config) => commands::index::command(config).await?,
        Command::Search(config) => commands::search::command(config).await?,
        Command::Tombstone(config) => commands::tombstone::command(config).await?,
        Command::Settings(config) => commands::settings::command(config).await?,
    }
    Ok(())
}

fn init_simple_logs(log_verbose_count: u8) {
    let mut builder = env_logger::Builder::new();
    match log_verbose_count {
        0 => builder.filter(Some("weise"), log::LevelFilter::Info),
        1 => builder.filter(None, log::LevelFilter::Info),
        2 => builder.filter(None, log::LevelFilter::Debug),
        _ => builder.filter(None, log::LevelFilter::Trace),
    };
    builder.init()
}
