use sha2::{Digest, Sha256};
use std::{cell::RefCell, io::{self, Cursor, Write}, rc::Rc};
use tdrive_storage::{plan_upload_parts, prepare_upload};

#[derive(Clone)]
struct Sink(Rc<RefCell<Vec<u8>>>);
impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

#[test]
fn upload_preparation_splits_and_hashes_exact_bytes() {
    let mut outputs = Vec::new();
    let result = prepare_upload(&mut Cursor::new(b"abcde"), 5, 2, |part| {
        assert_eq!(part.index as usize, outputs.len());
        let sink = Sink(Rc::new(RefCell::new(Vec::new())));
        outputs.push(sink.clone());
        Ok(sink)
    }).unwrap();
    assert_eq!(result.parts.iter().map(|p| (p.part.offset, p.part.size)).collect::<Vec<_>>(), [(0,2),(2,2),(4,1)]);
    assert_eq!(result.sha256, <[u8;32]>::from(Sha256::digest(b"abcde")));
    for (sink, expected) in outputs.iter().zip([b"ab".as_slice(),b"cd",b"e"]) {
        assert_eq!(sink.0.borrow().as_slice(), expected);
    }
    assert_eq!(result.parts[0].sha256, <[u8;32]>::from(Sha256::digest(b"ab")));
}

#[test]
fn invalid_plan_and_length_mismatch_do_not_return_receipt() {
    assert!(plan_upload_parts(0, 1).is_err());
    assert!(plan_upload_parts(1, 0).is_err());
    assert!(plan_upload_parts(100001, 1).is_err());
    assert!(plan_upload_parts(u64::MAX, 1).is_err());
    for bytes in [b"a".as_slice(), b"abc"] {
        assert!(prepare_upload(&mut Cursor::new(bytes), 2, 1, |_| Ok(io::sink())).is_err());
    }
    assert!(prepare_upload(&mut Cursor::new(b"ab"), 2, 1,
        |_| -> io::Result<io::Sink> { Err(io::Error::other("synthetic write failure")) }).is_err());
}
