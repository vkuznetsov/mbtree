use std::{marker::PhantomData, mem, ptr};

type Offset = u16;

const PAGE_SIZE: usize = 8192;
const OFFSET_SIZE: usize = mem::size_of::<Offset>();
const PAGE_HEADER_SIZE: usize = OFFSET_SIZE * 2;
const DATA_SIZE: usize = PAGE_SIZE - PAGE_HEADER_SIZE;
const MAX_OFFSET: Offset = PAGE_SIZE as Offset;
const MIN_OFFSET: Offset = PAGE_HEADER_SIZE as Offset;

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    NotEnoughSpace,
}

struct PageHeader {
    last_ref: Offset,
    last_element: Offset,
    element_pointers: [Offset; DATA_SIZE / 2],
}

struct Page<T: Ord> {
    _phantom: PhantomData<T>,
    data: [u8; PAGE_SIZE],
}

impl<T: Ord> Default for Page<T> {
    fn default() -> Self {
        let mut page = Page {
            _phantom: PhantomData,
            data: [0; PAGE_SIZE],
        };

        let hdr = page.mut_header();
        hdr.last_element = MAX_OFFSET;
        hdr.last_ref = MIN_OFFSET - 1;

        page
    }
}

impl<T: Ord> Page<T> {
    pub fn add(&mut self, item: T) -> Result<usize, Error> {
        let item_size = mem::size_of_val(&item);
        let hdr = self.mut_header();

        // We need a place for new item and for new element of element_pointers
        if item_size + OFFSET_SIZE > (hdr.last_element - hdr.last_ref) as usize {
            return Err(Error::NotEnoughSpace);
        }

        let idx = (hdr.last_ref - (MIN_OFFSET - 1)) as usize;

        hdr.last_element -= item_size as Offset;
        hdr.last_ref += 1;
        hdr.element_pointers[idx] = hdr.last_element;

        let offset = hdr.last_element as usize;

        unsafe {
            let addr = self.data.as_ptr().add(offset);
            ptr::write(addr as *mut T, item);
        };

        Ok(idx)
    }

    pub fn get(&self, idx: usize) -> &T {
        let offset: Offset = self.header().element_pointers[idx];
        unsafe { &*(self.data.as_ptr().add(offset as usize) as *const T) }
    }

    fn mut_header(&mut self) -> &mut PageHeader {
        unsafe { &mut *(self.data.as_mut_ptr() as *mut PageHeader) }
    }

    fn header(&self) -> &PageHeader {
        unsafe { &*(self.data.as_ptr() as *const PageHeader) }
    }
}

#[cfg(test)]
mod test {
    use std::mem;

    use super::{Error, Page, DATA_SIZE, PAGE_SIZE};

    #[test]
    fn size_test() {
        assert_eq!(mem::size_of::<Page<String>>(), PAGE_SIZE);
    }

    #[test]
    fn add_test() {
        let mut page: Page<String> = Page::default();

        let idx0 = page.add("Hello".to_owned()).unwrap();
        assert_eq!(idx0, 0);
        assert_eq!(page.get(idx0), "Hello");

        let idx1 = page.add("world!".to_owned()).unwrap();
        assert_eq!(idx1, 1);
        assert_eq!(page.get(idx1), "world!");
    }

    #[test]
    fn add_overflow_test() {
        const ITEM_SIZE: usize = DATA_SIZE / 2 - mem::size_of::<u16>();
        let mut page: Page<[u8; ITEM_SIZE]> = Page::default();

        let idx0 = page.add([0; ITEM_SIZE]).unwrap();
        assert_eq!(idx0, 0);

        let idx1 = page.add([0; ITEM_SIZE]).unwrap();
        assert_eq!(idx1, 1);

        assert_eq!(page.add([0; ITEM_SIZE]), Err(Error::NotEnoughSpace))
    }
}
