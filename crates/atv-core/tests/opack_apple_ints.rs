// Integer encodings as Apple's CoreUtils produces them (OPACKEncoderCreateData on macOS).
// Sized integers are signed there, so each size holds values up to its signed maximum;
// fake_atv.opack_pack sizes them unsigned, so these cases are not generated from it.

use atv_core::opack::{pack, unpack, Value};

#[test]
fn opack_ints_match_apple() {
    let cases: &[(u64, &str)] = &[
        (40, "3028"),
        (127, "307f"),
        (128, "318000"),
        (255, "31ff00"),
        (256, "310001"),
        (32767, "31ff7f"),
        (32768, "3200800000"),
        (65535, "32ffff0000"),
        (65536, "3200000100"),
        (2147483647, "32ffffff7f"),
        (2147483648, "330000008000000000"),
        (4294967295, "33ffffffff00000000"),
        (4294967296, "330000000001000000"),
    ];
    for (value, hex) in cases {
        let expected = hex::decode(hex).unwrap();
        assert_eq!(pack(&Value::Int(*value)), expected, "pack mismatch for {value}");
        let (unpacked, remaining) = unpack(&expected).unwrap();
        assert!(remaining.is_empty(), "trailing bytes for {value}");
        assert_eq!(unpacked, Value::Int(*value), "unpack mismatch for {value}");
    }
}
