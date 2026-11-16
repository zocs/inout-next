use share_native_slint::presentation::{parse_port, Locale};

#[test]
fn native_language_selection_obeys_scripts_and_does_not_read_extension_tokens_as_regions() {
    let cases = [
        ("en-US", Some(Locale::English)),
        ("zh-CN", Some(Locale::Simplified)),
        ("zh-TW", Some(Locale::Traditional)),
        ("zh_HK.UTF-8", Some(Locale::Traditional)),
        ("zh-Hans-TW", Some(Locale::Simplified)),
        ("zh-Hant-CN", Some(Locale::Traditional)),
        ("zh-x-hant-tw", Some(Locale::Simplified)),
        ("zh-CN-u-rg-twzzzz", Some(Locale::Simplified)),
        ("zh-Hant-x-hans", Some(Locale::Traditional)),
        ("zh-TW-u-ca-buddhist-x-hans", Some(Locale::Traditional)),
        ("en-u-ca-gregory-t-zh-hant", Some(Locale::English)),
        ("zh-u-ca-buddhist-u-rg-twzzzz", None),
        ("zh-x", None),
        ("zh--TW", None),
        ("zh-Hant-Hans", None),
        ("ja-JP", None),
    ];
    for (tag, expected) in cases {
        assert_eq!(Locale::from_tag(tag), expected, "{tag}");
    }
}

#[test]
fn port_input_accepts_automatic_selection_and_rejects_truncation_or_non_ascii_digits() {
    for (input, expected) in [("0", 0), ("5000", 5000), ("65535", 65535), (" 42 ", 42)] {
        assert_eq!(parse_port(input), Ok(expected));
    }
    for input in [
        "",
        "65536",
        "-1",
        "12.5",
        "5000x",
        "５０００",
        "999999999999999999999",
    ] {
        assert!(parse_port(input).is_err(), "{input}");
    }
}
