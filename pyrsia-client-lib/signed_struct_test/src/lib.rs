

#[cfg(test)]
mod tests {
    use signed_struct::signed_struct;

    #[signed_struct]
    struct Foo<'a> {
        foo: &'a str,
        bar: u32,
        zot: &'a str,
    }


    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
