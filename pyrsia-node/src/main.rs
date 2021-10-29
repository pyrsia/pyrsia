use std::io::prelude::{Read, Write};
use std::net::{TcpListener, TcpStream};

mod threading;
use threading::ThreadPool;

const PORT: &str = "7878";

fn main() {
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
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();
    println!("Request: {}", String::from_utf8_lossy(&buffer[..]));

    let response = "hi!";
    println!("Response: {}", response);
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
