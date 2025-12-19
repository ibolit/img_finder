use img_finder::library::config::Config;
use img_finder::library::lib::process_whole_task;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[clap(
        subcommand,
        help = "Some general help",
        long_help = r#"Sort and deduplicate your pictures on your hard drive"#
    )]
    subcommand: AppSubcommand,
    #[arg(long, short, help = "verbose")]
    verbose: bool,
}

#[derive(Debug, Subcommand, Clone)]
enum AppSubcommand {
    #[command(
        about = "short about",
        long_about = "Recursively walk the directory and create the .yaml files containing all the hashes and the files."
    )]
    Index {
        #[arg(long, short, help = "Some help")]
        dir: String,
    },
}

fn main() {
    let config = Config::new("config.yaml");
    let args = Args::parse();
    match args.subcommand {
        AppSubcommand::Index { dir } => {
            process_whole_task(
                &dir,
                config.image_formats,
                config.known_formats,
                config.skip_dirs,
                args.verbose,
            );
        }
    }
}
