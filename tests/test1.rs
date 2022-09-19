mod utils;

#[cfg(test)]
mod tests {
    use remote_object_derive::RemoteObject;

    use remote_object::*;

    use super::utils::*;

    #[test]
    fn it_works() {
        let mut v1 = 1usize;
        let mut v2 = 3usize;
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
        assert_eq!(v1, 3);
        assert_eq!(v2, 3);
    }

    #[test]
    fn it_works2() {
        let mut v1 = 1usize;
        let mut v2 = 3usize;
        std::thread::scope(|s| {
            let r1 = &mut v1;
            let r2 = &mut v2;
            let (mut p1, mut p2) = channel();
            s.spawn(move || {
                r1.sync(&mut p1, SyncPriority(3)).unwrap();
            });
            s.spawn(move || {
                r2.sync(&mut p2, SyncPriority(2)).unwrap();
            });
        });
        assert_eq!(v1, 1);
        assert_eq!(v2, 1);
    }

    #[test]
    fn test_derive_struct() {
        #[derive(RemoteObject)]
        struct Sendable(u64);
        let mut v1 = Sendable(1);
        let mut v2 = Sendable(3);
        std::thread::scope(|s| {
            let r1 = &mut v1;
            let r2 = &mut v2;
            let (mut p1, mut p2) = channel();
            s.spawn(move || {
                r1.sync(&mut p1, SyncPriority(3)).unwrap();
            });
            s.spawn(move || {
                r2.sync(&mut p2, SyncPriority(2)).unwrap();
            });
        });
        assert_eq!(v1.0, 1);
        assert_eq!(v2.0, 1);
    }


    #[test]
    fn test_string() {
        let mut s1 = "abc".to_owned();
        let mut s2 = "defg".to_owned();
        std::thread::scope(|s| {
            let r1 = &mut s1;
            let r2 = &mut s2;
            let (mut p1, mut p2) = channel();
            s.spawn(move || {
                r1.sync(&mut p1, SyncPriority(1)).unwrap();
            });
            s.spawn(move || {
                r2.sync(&mut p2, SyncPriority(2)).unwrap();
            });
        });
        assert_eq!(&s1, "defg");
        assert_eq!(&s2, "defg");
    }

    #[test]
    fn test_string_shorter() {
        let mut s1 = "abc".to_owned();
        let mut s2 = "de".to_owned();
        std::thread::scope(|s| {
            let r1 = &mut s1;
            let r2 = &mut s2;
            let (mut p1, mut p2) = channel();
            s.spawn(move || {
                r1.sync(&mut p1, SyncPriority(1)).unwrap();
            });
            s.spawn(move || {
                r2.sync(&mut p2, SyncPriority(2)).unwrap();
            });
        });
        assert_eq!(&s1, "de");
        assert_eq!(&s2, "de");
    }
}
