struct BidirChannel {
    bufs: [std::sync::Mutex<Vec<u8>>; 2],
}
pub struct Peer {
    chan: std::sync::Arc<BidirChannel>,
    bufidx: usize,
}
impl std::io::Read for Peer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut len = 0;
        while len == 0 {
            let mut lock = self.chan.bufs[self.bufidx].lock().unwrap();
            len = std::cmp::min(buf.len(), lock.len());
            if len != 0 {
                buf.copy_from_slice(&lock[0..len]);
                lock.rotate_left(len);
                let vec_len = lock.len();
                lock.truncate(vec_len - len);
                println!("Read: {buf:?}, now vec is: {:?}", &*lock);
            }
            drop(lock);
        }
        Ok(len)
    }
}
impl std::io::Write for Peer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut lock = self.chan.bufs[1 - self.bufidx].lock().unwrap();
        lock.extend_from_slice(buf);
        println!("Written: {buf:?}, now vec is: {:?}", &*lock);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub fn channel() -> (Peer, Peer) {
    let chan = std::sync::Arc::new(BidirChannel {
        bufs: [Default::default(), Default::default()],
    });
    let peer1 = Peer {
        chan: std::sync::Arc::clone(&chan),
        bufidx: 0,
    };
    let peer2 = Peer { chan, bufidx: 1 };
    (peer1, peer2)
}
