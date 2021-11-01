extern crate clap;
use clap::{App, Arg, ArgMatches};
use std::io::{self, Stdin};

fn main() {
    let mut authors: Vec<&'static str> = Vec::new();
    authors.push("Joeri Sykora <joeri@sertik.net>");
    authors.push("Elliott Frisch <elliottf@jfrog.com>");
    let matches: ArgMatches = App::new("Pyrsia Node")
        .version("0.1.0")
        .author(&*authors.join(", "))
        .about("Application to connect to and participate in the Pyrsia network")
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .takes_value(false)
                .required(false)
                .multiple(true)
                .help("Enables verbose output"),
        )
        .get_matches();
    let verbosity: u64 = matches.occurrences_of("verbose");

    println!("Pyrsia Node is now running!");
    if verbosity > 0 {
        println!("Verbosity Level: {}", verbosity.to_string())
    }
    println!("Press enter to exit...");

    let stdin: Stdin = io::stdin();
    let mut buffer: String = String::new();
    match stdin.read_line(&mut buffer) {
        Ok(n) => {
            if n > 1 {
                if verbosity > 0 {
                    println!("{} bytes read", n);
                }
                println!("{}", buffer);
            }
        }
        Err(error) => {
            println!("error: {}", error);
        }
    }
    println!("Pyrsia Node exited.");
}
