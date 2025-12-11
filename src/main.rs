mod converters;

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(version, about = "Convert trading reports to TraderVue format", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a trading report to CSV format
    Convert {
        /// Type of input report
        #[arg(short = 't', long, value_enum)]
        r#type: ReportType,

        /// Path to input XLSX file
        #[arg(short, long)]
        input: PathBuf,

        /// Path to output CSV file (defaults to input file with .csv extension)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, ValueEnum)]
enum ReportType {
    /// CQG Fill Report
    CQGFillReport,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert { r#type, input, output } => {
            if let Err(e) = handle_convert(r#type, input, output) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn handle_convert(
    report_type: ReportType,
    input: PathBuf,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Generate output path if not provided
    let output = output.unwrap_or_else(|| {
        input.with_extension("csv")
    });
    // Parse XLSX to Vec<Vec<String>>
    let rows = converters::parse_xlsx(&input)?;

    // Convert based on type
    let csv_rows = match report_type {
        ReportType::CQGFillReport => converters::cqg_fill_report::convert(rows)?,
    };

    // Write to CSV
    converters::write_csv(&output, csv_rows)?;

    println!("Successfully converted {} to {}", input.display(), output.display());
    Ok(())
}