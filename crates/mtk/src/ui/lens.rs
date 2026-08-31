/// A Lens allows focusing on a specific part `Inner` of a larger state `Outer`.
pub trait Lens<Outer: ?Sized, Inner: ?Sized> {
    fn get<'a>(&self, outer: &'a Outer) -> &'a Inner;
}

impl<Outer: ?Sized, Inner: ?Sized, F> Lens<Outer, Inner> for F
where
    F: Fn(&Outer) -> &Inner,
{
    fn get<'a>(&self, outer: &'a Outer) -> &'a Inner {
        (self)(outer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct User {
        name: String,
        age: u32,
    }

    fn use_lens<'a, O, I, L: Lens<O, I>>(lens: &L, outer: &'a O) -> &'a I {
        lens.get(outer)
    }

    #[test]
    fn test_fn_pointer_and_closure_lens() {
        let user = User {
            name: "Alice".to_string(),
            age: 30,
        };

        let name_lens: fn(&User) -> &String = |u| &u.name;
        let age_lens: fn(&User) -> &u32 = |u| &u.age;

        assert_eq!(use_lens(&name_lens, &user), "Alice");
        assert_eq!(*use_lens(&age_lens, &user), 30);
        assert_eq!(name_lens.get(&user), "Alice");
        assert_eq!(*age_lens.get(&user), 30);
    }

    fn get_name(u: &User) -> &String {
        &u.name
    }

    #[test]
    fn test_named_fn_lens() {
        let user = User {
            name: "Bob".to_string(),
            age: 25,
        };

        let name_fn: fn(&User) -> &String = get_name;
        assert_eq!(use_lens(&name_fn, &user), "Bob");
        assert_eq!(name_fn.get(&user), "Bob");
    }
}
