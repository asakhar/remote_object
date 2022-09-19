mod utils;

#[cfg(test)]
mod tests {
    use remote_object_derive::RemoteObject;

    use remote_object::*;

    use super::utils::*;
    #[test]
    fn test_derive_enum() {
        #[derive(RemoteObject, PartialEq, Debug)]
        enum Sendable {
            Opt1,
            Opt2(String),
            Opt3(i64, i64),
        }
        use Sendable::*;
        let mut v1 = Sendable::Opt1;
        let mut v2 = Sendable::Opt2("opt2".to_owned());
        std::thread::scope(|s| {
            let r1 = &mut v1;
            let r2 = &mut v2;
            let (mut p1, mut p2) = channel();
            s.spawn(move || {
                r1.sync(&mut p1, SyncPriority(1)).unwrap();
            });
            s.spawn(move || {
                r2.sync(&mut p2, SyncPriority(2)).unwrap();
            });
        });
        assert_eq!(v1, Sendable::Opt2("opt2".to_owned()));
        assert_eq!(v2, Sendable::Opt2("opt2".to_owned()));
    }
}
