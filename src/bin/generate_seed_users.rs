// Utility to generate seed users to populate the database

use std::{env, process};

use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use fake::faker::internet::en::{Password, Username};
use fake::Fake;
use uuid::Uuid;

/// Seed user
#[derive(Debug)]
pub struct SeedUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl SeedUser {
    /// Generate new random seed user
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Username().fake(),
            password: Password(32..33).fake(),
        }
    }

    /// Return PHC string for the provided password and a random salt
    pub fn hash_password(&self) -> String {
        let salt = SaltString::generate(&mut rand::thread_rng());
        Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string()
    }
}

/// Generate example seed users
#[allow(clippy::assigning_clones)]
fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg.starts_with('-')) {
        usage(&args[0]);
    }

    // Decide the course of action based on the number of arguments
    match args.len() {
        // Generate 10 sample seed users with random username and password
        1 => {
            for _ in 0..10 {
                let seeduser = SeedUser::generate();
                println!("{seeduser:?} PhcString: {}", seeduser.hash_password());
            }
        }

        // Generate a sample seed user with the specified username and a random password
        2 => {
            let mut seeduser = SeedUser::generate();
            seeduser.username = args[1].clone();
            println!("{seeduser:?} PhcString: {}", seeduser.hash_password());
        }

        // Generate a sample seed user with the specified username and password
        3 => {
            let mut seeduser = SeedUser::generate();
            seeduser.username = args[1].clone();
            seeduser.password = args[2].clone();
            println!("{seeduser:?} PhcString: {}", seeduser.hash_password());
        }

        // Print usage and exit
        _ => {
            usage(&args[0]);
        }
    };
}

/// Print usage information and exit
fn usage(prog: &str) {
    println!("Usage:");
    println!("{prog} [username] [password]");
    println!("\nExamples:");
    println!("{prog}");
    println!("{prog} admin everythinghastostartsomewhere");

    process::exit(1);
}
