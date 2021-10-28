use std::io::{self, Read};

fn main() {
    println!("Pyrsia Node is now running!");
    println!("Press enter to exit...");

    let stdin = io::stdin();
    for _b in stdin.bytes() {
        break;
    }

    println!("Pyrsia Node exited.");
}
