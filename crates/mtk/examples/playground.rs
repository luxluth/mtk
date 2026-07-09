fn main() {
    let cursor: Option<parley::editing::Cursor> = None;
    if let Some(c) = cursor {
        println!("{:?}", c);
    }
}
