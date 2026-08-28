use gbk_macro::{gb2312, gbk};

#[test]
fn ascii_passthrough() {
    const MSG: &[u8] = gbk!("hello");
    assert_eq!(MSG, b"hello");
}

#[test]
fn empty_string() {
    const MSG: &[u8] = gbk!("");
    assert_eq!(MSG, b"");
}

#[test]
fn user_example_start() {
    const MSG_START: &[u8] = gbk!("系统启动中...");
    assert_eq!(
        MSG_START,
        &[
            0xCF, 0xB5, 0xCD, 0xB3, 0xC6, 0xF4, 0xB6, 0xAF, 0xD6, 0xD0, 0x2E, 0x2E, 0x2E
        ]
    );
}

#[test]
fn user_example_error() {
    const MSG_ERROR: &[u8] = gbk!("传感器异常！");
    assert_eq!(
        MSG_ERROR,
        &[
            0xB4, 0xAB, 0xB8, 0xD0, 0xC6, 0xF7, 0xD2, 0xEC, 0xB3, 0xA3, 0xA3, 0xA1
        ]
    );
}

#[test]
fn gb2312_matches_gbk_for_common_chinese() {
    const A: &[u8] = gbk!("测试代码1");
    const B: &[u8] = gb2312!("测试代码1");
    assert_eq!(A, B);
}

#[test]
fn unicode_escape_in_literal() {
    const MSG: &[u8] = gbk!("\u{7CFB}统");
    assert_eq!(MSG, &[0xCF, 0xB5, 0xCD, 0xB3]);
}

#[test]
fn mixed_ascii_and_chinese() {
    const MSG: &[u8] = gbk!("OK-完成");
    assert_eq!(MSG, &[b'O', b'K', b'-', 0xCD, 0xEA, 0xB3, 0xC9]);
}
