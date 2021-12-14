use std::io::BufRead;

// Reads the first line from a BufRead
pub fn first_line<R>(mut rdr: R) -> String
where
    R: BufRead,
{
    let mut first_line: String = String::new();
    rdr.read_line(&mut first_line).expect("Unable to read line");
    first_line
}
