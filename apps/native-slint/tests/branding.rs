use share_native_slint::branding::{wordmark, Dot};

#[test]
fn a_renamed_product_uses_its_own_glyphs_and_letter_spacing() {
    let mark = wordmark("i1").expect("supported product name must get a matrix");
    assert_eq!(mark.columns, 11);
    // The lowercase i has a dot at (2, 0), no row 1 pixels and
    // a three-dot baseline. The second glyph begins at column 6.
    assert!(mark.dots.contains(&Dot { x: 2, y: 0 }));
    assert!(!mark.dots.iter().any(|dot| dot.x < 5 && dot.y == 1));
    assert!(mark.dots.contains(&Dot { x: 1, y: 6 }));
    assert!(mark.dots.contains(&Dot { x: 3, y: 6 }));
    assert!(mark.dots.contains(&Dot { x: 8, y: 0 }));
    assert!(mark.dots.contains(&Dot { x: 7, y: 1 }));
    assert!(mark
        .dots
        .iter()
        .all(|dot| (0..11).contains(&dot.x) && (0..7).contains(&dot.y)));
    assert_eq!(wordmark("inout").unwrap().columns, 29);
    assert_eq!(wordmark("AbCd0123").unwrap().columns, 47);
}

#[test]
fn unreadable_or_unsupported_brand_names_keep_their_plain_text_fallback() {
    for name in [
        "",
        "a-long-name",
        "123456789",
        "文件",
        "hello world",
        "inout🦀",
    ] {
        assert!(wordmark(name).is_none(), "{name}");
    }
}
