/// A Lens allows focusing on a specific part (`Inner`) of a larger state (`Outer`).
pub trait Lens<Outer, Inner> {
    fn get<'a>(&self, outer: &'a Outer) -> &'a Inner;
}
