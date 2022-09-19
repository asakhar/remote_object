use std::{
    fmt::Display,
    io::{Read, Write},
    mem::size_of,
    str::Utf8Error,
    string::FromUtf8Error,
};

#[derive(Debug, Clone, Copy)]
pub enum SyncError {
    ReadError,
    WriteError,
    EquPrior(SyncPriority),
    Utf8Error(Utf8Error),
}
impl Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SyncError::*;
        match *self {
            ReadError => f.write_str("ReadError"),
            WriteError => f.write_str("WriteError"),
            EquPrior(prior) => f.write_fmt(format_args!("EquPrior({prior})")),
            Utf8Error(error) => f.write_fmt(format_args!("{error}")),
        }
    }
}
impl From<std::io::Error> for SyncError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::Interrupted => Self::WriteError,
            std::io::ErrorKind::UnexpectedEof => Self::ReadError,
            _ => unreachable!("Enexpected error"),
        }
    }
}
impl From<FromUtf8Error> for SyncError {
    fn from(error: FromUtf8Error) -> Self {
        Self::Utf8Error(error.utf8_error())
    }
}
impl std::error::Error for SyncError {}
pub type SyncResult<T> = Result<T, SyncError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SyncPriority(pub u64);
impl Display for SyncPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("SyncPriority({})", self.0))
    }
}

pub trait RemoteObject {
    fn sync<C: Read + Write>(&mut self, channel: &mut C, priority: SyncPriority) -> SyncResult<()> {
        let dir = sync_direction(channel, priority)?;
        match dir {
            SyncDirection::Pull => self.pull(channel),
            SyncDirection::Push => self.push(channel),
        }
    }
    fn pull<C: Read>(&mut self, channel: &mut C) -> SyncResult<()>;
    fn push<C: Write>(&self, channel: &mut C) -> SyncResult<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    Pull,
    Push,
}

pub fn sync_direction<C: Read + Write>(
    channel: &mut C,
    priority: SyncPriority,
) -> SyncResult<SyncDirection> {
    channel.write_all(&priority.0.to_le_bytes())?;
    let mut priority_buffer = [0u8; size_of::<SyncPriority>()];
    channel.read_exact(&mut priority_buffer)?;
    let peer_priority = SyncPriority(u64::from_le_bytes(priority_buffer));
    if peer_priority == priority {
        Err(SyncError::EquPrior(priority))
    } else {
        Ok(if peer_priority > priority {
            SyncDirection::Pull
        } else {
            SyncDirection::Push
        })
    }
}

macro_rules! impl_ro_for_primitive {
    ($prim:ident) => {
        impl RemoteObject for $prim {
            fn pull<C: Read>(&mut self, channel: &mut C) -> SyncResult<()> {
                channel.read_exact(unsafe {
                    std::slice::from_raw_parts_mut(self as *mut _ as *mut _, size_of::<Self>())
                })?;
                Ok(())
            }
            fn push<C: Write>(&self, channel: &mut C) -> SyncResult<()> {
                channel.write_all(unsafe {
                    std::slice::from_raw_parts(self as *const _ as *const _, size_of::<Self>())
                })?;
                Ok(())
            }
        }
    };
}

impl RemoteObject for u8 {
    fn pull<C: Read>(&mut self, channel: &mut C) -> SyncResult<()> {
        channel.read_exact(std::slice::from_mut(self))?;
        Ok(())
    }
    fn push<C: Write>(&self, channel: &mut C) -> SyncResult<()> {
        channel.write_all(std::slice::from_ref(self))?;
        Ok(())
    }
}

impl_ro_for_primitive!(i8);
impl_ro_for_primitive!(u16);
impl_ro_for_primitive!(i16);
impl_ro_for_primitive!(u32);
impl_ro_for_primitive!(i32);
impl_ro_for_primitive!(u64);
impl_ro_for_primitive!(i64);
impl_ro_for_primitive!(usize);
impl_ro_for_primitive!(isize);
impl_ro_for_primitive!(f32);
impl_ro_for_primitive!(f64);

macro_rules! impl_ro_for_slices {
    ($prim:ident) => {
        impl<const N: usize> RemoteObject for [$prim; N] {
            fn pull<C: Read>(&mut self, channel: &mut C) -> SyncResult<()> {
                channel.read_exact(unsafe {
                    std::slice::from_raw_parts_mut(
                        self.as_mut_ptr() as *mut _,
                        size_of::<$prim>() * N,
                    )
                })?;
                Ok(())
            }
            fn push<C: Write>(&self, channel: &mut C) -> SyncResult<()> {
                channel.write_all(unsafe {
                    std::slice::from_raw_parts(self.as_ptr() as *mut _, size_of::<$prim>() * N)
                })?;
                Ok(())
            }
        }
    };
}

impl_ro_for_slices!(u8);
impl_ro_for_slices!(i8);
impl_ro_for_slices!(u16);
impl_ro_for_slices!(i16);
impl_ro_for_slices!(u32);
impl_ro_for_slices!(i32);
impl_ro_for_slices!(u64);
impl_ro_for_slices!(i64);
impl_ro_for_slices!(usize);
impl_ro_for_slices!(isize);
impl_ro_for_slices!(f32);
impl_ro_for_slices!(f64);

impl RemoteObject for String {
    fn pull<C: Read>(&mut self, channel: &mut C) -> SyncResult<()> {
        let mut len_buf = [0u8; size_of::<usize>()];
        channel.read_exact(&mut len_buf)?;
        let len = usize::from_le_bytes(len_buf);
        let mut string_buf = vec![0u8; len];
        channel.read_exact(&mut string_buf)?;
        *self = String::from_utf8(string_buf)?;
        Ok(())
    }

    fn push<C: Write>(&self, channel: &mut C) -> SyncResult<()> {
        let bytes = self.as_bytes();
        channel.write_all(&bytes.len().to_le_bytes())?;
        channel.write_all(bytes)?;
        Ok(())
    }
}

impl<T: RemoteObject + Default> RemoteObject for Vec<T> {
    fn pull<C: Read>(&mut self, channel: &mut C) -> SyncResult<()> {
        let mut len_buf = [0u8; size_of::<usize>()];
        channel.read_exact(&mut len_buf)?;
        let len = usize::from_le_bytes(len_buf);
        self.resize_with(len, Default::default);
        self.iter_mut().map(|item| item.pull(channel)).collect()
    }

    fn push<C: Write>(&self, channel: &mut C) -> SyncResult<()> {
        channel.write_all(&self.len().to_le_bytes())?;
        self.iter().map(|item| item.push(channel)).collect()
    }
}
