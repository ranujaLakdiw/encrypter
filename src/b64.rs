//! Base64 the way the page has always exchanged it: standard alphabet, padded on
//! the way out, and forgiving on the way in (white space and missing padding are
//! fine, as they were with `atob`).

use base64::{
    alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
    Engine,
};

const ENGINE: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_padding_mode(DecodePaddingMode::Indifferent)
        .with_decode_allow_trailing_bits(true),
);

pub fn encode(bytes: &[u8]) -> String {
    ENGINE.encode(bytes)
}

pub fn decode(text: &str) -> Option<Vec<u8>> {
    if text.bytes().any(|b| b.is_ascii_whitespace()) {
        let clean: String = text.chars().filter(|c| !c.is_ascii_whitespace()).collect();
        ENGINE.decode(clean).ok()
    } else {
        ENGINE.decode(text).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forgiving_decode() {
        assert_eq!(decode("aGk=").unwrap(), b"hi");
        assert_eq!(decode("aGk").unwrap(), b"hi");
        assert_eq!(decode(" aG\nk= ").unwrap(), b"hi");
        assert!(decode("a*k=").is_none());
        assert_eq!(encode(b"hi"), "aGk=");
    }
}
