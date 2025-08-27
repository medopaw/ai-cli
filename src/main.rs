mod cli;
mod config;
mod ai_client;
mod git_ops;
mod history;
mod utils;
mod commands;

// use clap::Parser;
// use cli::{Cli, Commands};
use anyhow::Result;
use commands::*;

#[tokio::main]
async fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    
    // Handle no arguments or unrecognized commands
    if args.len() == 1 {
        show_help();
        return Ok(());
    }

    // Parse command manually to handle unrecognized commands
    let subcommand = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let remaining_args: Vec<String> = args.iter().skip(2).map(|s| s.clone()).collect();

    match subcommand {
        "help" => show_help(),
        "ask" => {
            if remaining_args.is_empty() {
                eprintln!("Error: 'ask' command requires a question");
                show_help();
                return Ok(());
            }
            let question = remaining_args.join(" ");
            handle_ask(&question).await?;
        }
        "chat" => handle_chat().await?,
        "commit" => {
            let all = remaining_args.contains(&"all".to_string());
            handle_commit(all).await?;
        }
        "push" => {
            let force = remaining_args.contains(&"force".to_string());
            handle_push(force).await?;
        }
        "publish" => handle_publish().await?,
        "fix" => {
            let user_context = remaining_args.join(" ");
            handle_fix(&user_context).await?;
        }
        _ => {
            eprintln!("Error: Unknown command '{}'", subcommand);
            show_help();
        }
    }

    Ok(())
}

fn show_help() {
    println!("ai - Personal AI CLI tool");
    println!();
    println!("USAGE:");
    println!("    ai <COMMAND>");
    println!();
    println!("COMMANDS:");
    println!("    help       Display this help message");
    println!("    ask        Ask AI a single question");
    println!("    chat       Start interactive chat session");
    println!("    commit     Commit changes with AI-generated message");
    println!("    push       Push changes to remote repository");
    println!("    publish    Publish project to appropriate registry");
    println!("    fix        Analyze terminal history and fix the last error");
    println!();
    println!("EXAMPLES:");
    println!("    ai ask \"How do I write a Rust function?\"");
    println!("    ai chat");
    println!("    ai commit all");
    println!("    ai push force");
    println!("    ai fix");
    println!("    ai fix \"cargo build failed with linking error\"");
}




