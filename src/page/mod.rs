use std::{marker::PhantomData, mem, ptr};

type Offset = u16;

const PAGE_SIZE: usize = 8192;
const PAGE_HEADER_SIZE: usize = mem::size_of::<Offset>() * 2;
const DATA_SIZE: usize = PAGE_SIZE - PAGE_HEADER_SIZE;
const MAX_OFFSET: Offset = PAGE_SIZE as Offset;

#[derive(Debug)]
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

        page.mut_header().last_element = MAX_OFFSET;
        page
    }
}

impl<T: Ord> Page<T> {
    pub fn add(&mut self, item: T) -> Result<(), Error> {
        let item_size = mem::size_of_val(&item);
        let hdr = self.mut_header();

        if item_size > (hdr.last_element - hdr.last_ref) as usize {
            return Err(Error::NotEnoughSpace);
        }

        // Как посчитать индекс в element_pointers?
        let idx = 12;

        hdr.last_element -= item_size as Offset;
        hdr.element_pointers[idx] = hdr.last_element;

        unsafe {
            let addr = self.data.as_ptr().add(PAGE_SIZE).sub(item_size);
            ptr::write(addr as *mut T, item);
        };

        Ok(())
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

    use super::{Page, PAGE_SIZE};

    #[test]
    fn size_test() {
        assert_eq!(mem::size_of::<Page<String>>(), PAGE_SIZE);
    }

    #[test]
    fn add_test() {
        let mut page: Page<String> = Page::default();
        page.add("Hello!".to_owned()).unwrap();
        assert_eq!(page.get(12), "Hello!");
    }
}
