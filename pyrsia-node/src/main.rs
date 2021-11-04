extern crate clap;
use clap::{App, Arg, ArgMatches};
use std::io::prelude::{BufRead, Write};
use std::io::BufReader;
use std::net::{TcpListener, TcpStream};

mod threading;
use threading::ThreadPool;

const PORT: &str = "7878";

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

    if verbosity > 0 {
        println!("Verbosity Level: {}", verbosity.to_string())
    }

    let listener = TcpListener::bind(format!("127.0.0.1:{}", PORT)).unwrap();
    println!("Pyrsia Node is now listening on port {}!", PORT);

    let threadpool = ThreadPool::new(16).unwrap_or_else(|error| {
        panic!("Error creating thread pool: {:?}", error.to_string());
    });

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        threadpool.execute(|| {
            handle_connection(stream);
        });
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut reader = BufReader::new(&stream);

    let mut request = String::new();
    reader.read_line(&mut request).unwrap();
    let request = request.trim_end_matches(|c| c == '\n' || c == '\r');

    println!("Request: '{}'", request);

    let response = "hi!";
    println!("Response: {}", response);
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
