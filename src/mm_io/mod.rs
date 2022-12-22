use std::{
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    marker::PhantomData,
    mem,
    os::unix::prelude::AsRawFd,
    path::Path,
    ptr,
};

use libc::c_void;

const MIN_SIZE: usize = 65536;

pub struct Mapping<T: Sized + Default> {
    _type_marker: PhantomData<T>,
    size: usize,
    file: File,
    mm: *mut u8,
}

#[derive(Debug)]
pub enum Error<'a> {
    IoErr(std::io::Error, &'a str),
    MmapErr(&'a str),
    TooLargeFile(u64),
    IndexOutOfBounds,
}

impl<T: Sized + Default> Mapping<T> {
    pub fn new(filepath: &Path) -> Result<Self, Error> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filepath)
            .map_err(|e| Error::IoErr(e, "can't open file"))?;

        let size = file.metadata().unwrap().len();

        if size > usize::MAX as u64 {
            return Err(Error::TooLargeFile(size));
        }

        let mut size = size as usize;

        if size < MIN_SIZE {
            enlarge_file(&mut file, MIN_SIZE)?;
            size = MIN_SIZE
        }

        let mm = mmap(&file, size)?;

        Ok(Mapping {
            size,
            file,
            mm,
            _type_marker: PhantomData,
        })
    }

    pub fn insert_at(&mut self, value: &T, idx: usize) -> Result<(), Error> {
        let vsize = mem::size_of::<T>();
        let p = idx * vsize;

        self.maybe_enlarge(p + vsize)?;

        unsafe {
            ptr::copy_nonoverlapping((value as *const T) as *const u8, self.mm.add(p), vsize);
        }

        Ok(())
    }

    pub fn read_at(&self, idx: usize) -> Result<T, Error> {
        let vsize = mem::size_of::<T>();
        let p = idx * vsize;
        let mut value: T = T::default();

        dbg!(p);

        if p >= self.size {
            return Err(Error::IndexOutOfBounds);
        }

        unsafe {
            let pv = &mut value as *mut T;
            // let pv = &value
            ptr::copy_nonoverlapping(self.mm.add(p), pv as *mut u8, vsize);
        }

        Ok(value)
    }

    fn maybe_enlarge(&mut self, desired_size: usize) -> Result<(), Error<'static>> {
        if self.size < desired_size {
            let new_size = next_size(desired_size, mem::size_of::<T>());

            munmap(self.mm, self.size)?;
            enlarge_file(&mut self.file, new_size)?;
            self.mm = mmap(&self.file, new_size)?;
            self.size = new_size;
        }

        Ok(())
    }
}

impl<T: Sized + Default> Drop for Mapping<T> {
    fn drop(&mut self) {
        munmap(self.mm, self.size).unwrap();
    }
}

fn next_size(size: usize, block_size: usize) -> usize {
    (size as f32 * 1.618).ceil() as usize / block_size * block_size
}

fn enlarge_file(file: &mut File, required_size: usize) -> Result<(), Error<'static>> {
    file.seek(SeekFrom::Start(required_size as u64))
        .map_err(|e| Error::IoErr(e, "can't enlarge file"))?;
    file.write_all(&[0])
        .map_err(|e| Error::IoErr(e, "can't enlarge file"))?;

    Ok(())
}

fn mmap(file: &File, len: usize) -> Result<*mut u8, Error<'static>> {
    let data = unsafe {
        libc::mmap(
            ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            file.as_raw_fd(),
            0,
        )
    };

    if data == libc::MAP_FAILED {
        return Err(Error::MmapErr(
            "Could not access data from memory mapped file",
        ));
    }

    Ok(data as *mut u8)
}

fn munmap(mm: *mut u8, len: usize) -> Result<(), Error<'static>> {
    unsafe {
        if libc::munmap(mm as *mut c_void, len) != 0 {
            return Err(Error::MmapErr("Unable unmap"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::{fs, mem, path::Path};

    use super::{Error, Mapping, MIN_SIZE};

    use rand::{distributions::Alphanumeric, Rng};

    #[derive(Debug)]
    struct TestPage {
        v: i32,
    }

    impl Default for TestPage {
        fn default() -> Self {
            Self {
                v: Default::default(),
            }
        }
    }

    struct TestFile {
        path_str: String,
    }

    #[test]
    fn mmap_test() {
        let (v1, idx1) = (1534, 0);
        let (v2, idx2) = (23423, MIN_SIZE / mem::size_of::<TestPage>());
        let (v3, idx3) = (87, 1051677);
        let test_file = TestFile::new();
        let test_file_path = test_file.path();

        {
            let mut mapping = Mapping::new(test_file_path).expect("can't create mapping");
            mapping
                .insert_at(&TestPage { v: v1 }, idx1)
                .expect("can't insert data");
            mapping
                .insert_at(&TestPage { v: v2 }, idx2)
                .expect("can't insert data");
            mapping
                .insert_at(&TestPage { v: v3 }, idx3)
                .expect("can't insert data");
        }

        {
            let mapping: Mapping<TestPage> =
                Mapping::new(test_file_path).expect("can't create mapping");

            let p = mapping.read_at(idx1).expect("can't read from mapping");
            assert_eq!(p.v, v1);

            let p = mapping.read_at(idx2).expect("can't read from mapping");
            assert_eq!(p.v, v2);

            let p = mapping.read_at(idx3).expect("can't read from mapping");
            assert_eq!(p.v, v3);
        }
    }

    #[test]
    fn mmap_test_index_of_bounds() {
        let mapping: Mapping<TestPage> =
            Mapping::new(TestFile::new().path()).expect("can't create mapping");

        assert!(matches!(
            mapping.read_at(MIN_SIZE / mem::size_of::<TestPage>()),
            Err(Error::IndexOutOfBounds)
        ));
    }

    impl TestFile {
        fn new() -> Self {
            let random_str: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(8)
                .map(char::from)
                .collect();

            // let path = Path::new(&format!("test_{}.db", random_str));
            let path_str = format!("test_{}.db", random_str);
            Self { path_str }
        }

        fn path(&self) -> &Path {
            Path::new(&self.path_str)
        }
    }

    impl Drop for TestFile {
        fn drop(&mut self) {
            fs::remove_file(self.path()).unwrap();
        }
    }
}
