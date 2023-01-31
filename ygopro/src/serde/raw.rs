pub unsafe fn serialize<'ser, T: Sized>(message: &'ser T) -> &'ser [u8] {
    std::slice::from_raw_parts(
        (message as *const T) as *const u8,
        std::mem::size_of::<T>()
    )
}

pub unsafe fn deserialize<'de, T: Sized>(data: &[u8]) -> &'de T {
    std::mem::transmute(data.as_ptr() as *const _)
}

#[test]
fn test_unsafe_searilize() {
    #[repr(C)]
    struct T {
        i: i32,
        j: String,
        k: [u8; 4]
    }
    let t = T { i: 999, j: "test".to_string(), k: [5,6,7,8] };
    let bytes = unsafe { serialize(&t) };
    let t2: &T = unsafe { deserialize(bytes) };
    assert_eq!(t2.i, 999);
    assert_eq!(t2.j, "test");
    assert_eq!(t2.k, [5,6,7,8])
}