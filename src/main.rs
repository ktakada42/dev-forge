mod banner;
mod repl;
mod tools;

use std::io::{self, IsTerminal, Read};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "forge", about = "dev-forge: developer utility tools")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Unix timestamp ↔ datetime conversion
    Timestamp {
        /// Timezone for output (e.g. Asia/Tokyo, UTC, +09:00)
        #[arg(long)]
        tz: Option<String>,
        value: Option<String>,
    },
    /// Base64 encode/decode
    Base64 {
        #[command(subcommand)]
        action: Base64Action,
    },
    /// URL encode/decode
    Url {
        #[command(subcommand)]
        action: UrlAction,
    },
    /// JWT decode (no signature verification)
    Jwt {
        #[command(subcommand)]
        action: JwtAction,
    },
}

#[derive(Subcommand)]
enum Base64Action {
    /// Encode to Base64
    Encode { value: Option<String> },
    /// Decode from Base64
    Decode { value: Option<String> },
}

#[derive(Subcommand)]
enum UrlAction {
    /// URL encode
    Encode { value: Option<String> },
    /// URL decode
    Decode { value: Option<String> },
}

#[derive(Subcommand)]
enum JwtAction {
    /// Decode JWT header and payload
    Decode { value: Option<String> },
}

fn get_value(opt: Option<String>) -> Result<String, String> {
    if let Some(v) = opt {
        return Ok(v);
    }
    if !io::stdin().is_terminal() {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|e| e.to_string())?;
        Ok(input.trim_end_matches('\n').to_string())
    } else {
        Err("No input provided. Pass a value as argument or pipe via stdin.".to_string())
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => repl::run(),
        Some(Commands::Timestamp { tz, value }) => match get_value(value) {
            Ok(v) => match tools::timestamp::convert(&v, tz.as_deref()) {
                Ok(result) => println!("{}", result),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        Some(Commands::Base64 { action }) => match action {
            Base64Action::Encode { value } => match get_value(value) {
                Ok(v) => println!("{}", tools::base64::encode(&v)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            },
            Base64Action::Decode { value } => match get_value(value) {
                Ok(v) => match tools::base64::decode(&v) {
                    Ok(result) => println!("{}", result),
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            },
        },
        Some(Commands::Url { action }) => match action {
            UrlAction::Encode { value } => match get_value(value) {
                Ok(v) => println!("{}", tools::url::encode(&v)),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            },
            UrlAction::Decode { value } => match get_value(value) {
                Ok(v) => match tools::url::decode(&v) {
                    Ok(result) => println!("{}", result),
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            },
        },
        Some(Commands::Jwt { action }) => match action {
            JwtAction::Decode { value } => match get_value(value) {
                Ok(v) => match tools::jwt::decode(&v) {
                    Ok(result) => println!("{}", result),
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            },
        },
    }
}
