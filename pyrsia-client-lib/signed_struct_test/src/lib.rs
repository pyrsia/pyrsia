#[cfg(test)]
mod tests {
    use signed_struct::signed_struct;

    #[signed_struct]
    struct Foo<'a> {
        foo: String,
        bar: u32,
        zot: &'a str,
    }

    #[test]
    fn test_generated_methods() {
        let foo = Foo::new(String::from("abc"), 234, &"qwerty");
    }
}
