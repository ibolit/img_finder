use img_finder::library::index::{get_info, process_whole_task, set_datetime};
use img_finder::library::stats::symlink_non_date;
use img_finder::library::{config::Config, stats::stats};

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
        #[arg(long, short, help = "File to write the resulting yamls to")]
        output: Option<String>,
    },
    Stats {
        #[arg(long, short, help = "Yaml file to analyze")]
        input: String,
    },
    Symlink {
        #[arg(long, short, help = "Yaml file to analyze")]
        input: String,
        #[arg(long, short, help = "Where to put the created symlinks")]
        output: String,
    },
    SetDate {
        // #[arg(long, short, help = "Image to COPY the datetime FROM")]
        input: String,
        // #[arg(long, short, help = "Image to SET the datetime TO")]
        date: String,
    },
    Info {
        input: String,
    },
}

fn main() {
    let config = Config::new("config.yaml");
    let args = Args::parse();
    match args.subcommand {
        AppSubcommand::Index { dir, output } => {
            let output = output.unwrap_or(dir.clone());
            process_whole_task(
                &dir,
                &output,
                config.image_formats,
                config.known_formats,
                config.skip_dirs,
                args.verbose,
            );
        }
        AppSubcommand::Stats { input } => {
            stats(&input);
        }
        AppSubcommand::Symlink { input, output } => {
            symlink_non_date(&input, &output, config.screenshot_resolutions);
        }
        AppSubcommand::SetDate { input, date } => {
            set_datetime(&input, &date);
        }
        AppSubcommand::Info { input } => {
            get_info(&input);
        }
    }
}
