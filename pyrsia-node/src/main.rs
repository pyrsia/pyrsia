extern crate clap;
use clap::{App, Arg, ArgMatches};
use std::io::{self, Read};

fn main() {
    let matches: ArgMatches = App::new("Pyrsia Node")
    .version("0.1.0")
    .author("Joeri Sykora <joeri@sertik.net>, Elliott Frisch <elliottf@jfrog.com>")
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

    let stdin = io::stdin();
    for _b in stdin.bytes() {
        break;
    }

    println!("Pyrsia Node exited.");
}
