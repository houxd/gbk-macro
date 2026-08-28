//! 编译期将 UTF-8 字符串转为 GBK / GB2312 字节数组。
//!
//! 源码与编辑器使用 UTF-8，而部分单片机外设（串口屏、热敏打印机等）只接受
//! GB2312/GBK。本 crate 在 **编译期** 完成转码，生成的是普通 `&[u8]` 常量，
//! 目标固件里既不依赖转码表，也不需要运行时转换。
//!
//! # 示例
//!
//! ```
//! use gbk_macro::gbk;
//!
//! const MSG_START: &[u8] = gbk!("系统启动中...");
//! const MSG_ERROR: &[u8] = gbk!("传感器异常！");
//!
//! assert_eq!(MSG_START, &[0xCF, 0xB5, 0xCD, 0xB3, 0xC6, 0xF4, 0xB6, 0xAF, 0xD6, 0xD0, b'.', b'.', b'.']);
//! assert_eq!(MSG_ERROR, &[0xB4, 0xAB, 0xB8, 0xD0, 0xC6, 0xF7, 0xD2, 0xEC, 0xB3, 0xA3, 0xA3, 0xA1]);
//! ```
//!
//! 在 `no_std` 嵌入式 crate 中直接依赖即可：过程宏在宿主机上执行，
//! 展开结果不含任何运行时依赖。

use encoding_rs::{EncoderResult, GBK};
use proc_macro::TokenStream;
use quote::quote;
use syn::{Error, LitStr, Result, parse_macro_input};

/// 将 UTF-8 字符串字面量编码为 GBK 字节切片。
///
/// 无法用 GBK 表示的字符（例如 emoji）会在编译期报错。
///
/// # 示例
///
/// ```
/// use gbk_macro::gbk;
///
/// const MSG: &[u8] = gbk!("系统启动中...");
/// assert_eq!(MSG[0], 0xCF);
/// ```
#[proc_macro]
pub fn gbk(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    expand(&lit, EncodingKind::Gbk).unwrap_or_else(|e| e.to_compile_error().into())
}

/// 将 UTF-8 字符串字面量编码为 GB2312（EUC-CN）字节切片。
///
/// GB2312 是 GBK 的子集。编码结果与 `gbk!` 对常用汉字相同，但若出现仅属于
/// GBK 的字符（例如部分生僻字），会在编译期报错，避免串口屏显示乱码。
///
/// # 示例
///
/// ```
/// use gbk_macro::gb2312;
///
/// const MSG: &[u8] = gb2312!("传感器异常！");
/// assert_eq!(MSG, gbk_macro::gbk!("传感器异常！"));
/// ```
#[proc_macro]
pub fn gb2312(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    expand(&lit, EncodingKind::Gb2312).unwrap_or_else(|e| e.to_compile_error().into())
}

#[derive(Clone, Copy)]
enum EncodingKind {
    Gbk,
    Gb2312,
}

fn expand(lit: &LitStr, kind: EncodingKind) -> Result<TokenStream> {
    let utf8 = lit.value();
    let bytes = utf8_to_gbk(&utf8).map_err(|err| match err {
        EncodeError::Unmappable(ch) => Error::new(
            lit.span(),
            format!("字符 {:?} (U+{:04X}) 无法编码为 GBK", ch, ch as u32),
        ),
        EncodeError::Internal => Error::new(lit.span(), "GBK 编码缓冲区不足（内部错误）"),
    })?;

    if matches!(kind, EncodingKind::Gb2312) {
        if let Some((lead, trail)) = first_non_gb2312_pair(&bytes) {
            return Err(Error::new(
                lit.span(),
                format!(
                    "字符串可编码为 GBK，但不属于 GB2312（双字节 {:#04X} {:#04X}）",
                    lead, trail
                ),
            ));
        }
    }

    Ok(quote! { &[#(#bytes),*] }.into())
}

#[derive(Debug)]
enum EncodeError {
    Unmappable(char),
    Internal,
}

fn utf8_to_gbk(s: &str) -> core::result::Result<Vec<u8>, EncodeError> {
    let mut encoder = GBK.new_encoder();
    let cap = encoder
        .max_buffer_length_from_utf8_without_replacement(s.len())
        .unwrap_or_else(|| s.len().saturating_mul(4).saturating_add(16));
    let mut dst = vec![0u8; cap];
    let (result, _read, written) = encoder.encode_from_utf8_without_replacement(s, &mut dst, true);
    match result {
        EncoderResult::InputEmpty => {
            dst.truncate(written);
            Ok(dst)
        }
        EncoderResult::OutputFull => Err(EncodeError::Internal),
        EncoderResult::Unmappable(ch) => Err(EncodeError::Unmappable(ch)),
    }
}

/// GB2312（EUC-CN）：ASCII 为单字节 0x00–0x7F；汉字为双字节，首字节 0xA1–0xF7、
/// 次字节 0xA1–0xFE，且不包括空闲区 10–15（首字节 0xAA–0xAF）。
fn first_non_gb2312_pair(bytes: &[u8]) -> Option<(u8, u8)> {
    let mut i = 0;
    while i < bytes.len() {
        let lead = bytes[i];
        if lead < 0x80 {
            i += 1;
            continue;
        }
        let Some(&trail) = bytes.get(i + 1) else {
            return Some((lead, 0));
        };
        let area = lead.wrapping_sub(0xA0);
        let valid = (0xA1..=0xF7).contains(&lead)
            && (0xA1..=0xFE).contains(&trail)
            && !(10..=15).contains(&area);
        if !valid {
            return Some((lead, trail));
        }
        i += 2;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emoji_cannot_encode() {
        assert!(matches!(utf8_to_gbk("😀"), Err(EncodeError::Unmappable(_))));
    }

    #[test]
    fn gbk_only_char_is_not_gb2312() {
        let bytes = utf8_to_gbk("镕").expect("镕 属于 GBK");
        assert_eq!(bytes, &[0xE9, 0x46]);
        assert!(first_non_gb2312_pair(&bytes).is_some());
    }
}
