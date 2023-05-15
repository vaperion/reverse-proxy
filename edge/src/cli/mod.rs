use clap::{Parser, Subcommand};

pub mod add_user;
pub mod delete_user;
pub mod serve;

#[derive(Parser, Debug)]
#[command(name = "fast-reverse-proxy")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A blazing fast ngrok alternative written in Rust")]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the edge server and listen for incoming connections
    Serve {},
    /// Add a new user and generate a secret key for them
    AddUser {
        name: String,
        max_tunnels: Option<usize>,
    },
    /// Delete an existing user by their name or secret key
    DeleteUser { name_or_key: String },
}
