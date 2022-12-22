## Educational implementation of persistent B-tree (WIP)

B-tree consisting of fixed-size pages that can store variable-length data.
Pages are stored to disk by writing to memory-mapped file directly into the Page Cache,
avoiding redundant mem copy from/to user/kernel space.

### TODO
- [x] Memory Mapping
- [ ] Page
- [ ] B-tree
