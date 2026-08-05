pub(super) fn same_name(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

pub(super) fn is_raw_text(name: &str) -> bool {
    [
        "script",
        "style",
        "title",
        "textarea",
        "xmp",
        "iframe",
        "noembed",
        "noframes",
        "plaintext",
    ]
    .iter()
    .any(|raw| same_name(name, raw))
}

pub(super) fn is_inert(name: &str) -> bool {
    ["template", "noscript"]
        .iter()
        .any(|raw| same_name(name, raw))
}

pub(super) fn is_void(name: &str) -> bool {
    [
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
        "source", "track", "wbr",
    ]
    .iter()
    .any(|void| same_name(name, void))
}
