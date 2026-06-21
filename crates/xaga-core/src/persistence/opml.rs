use quick_xml::escape::{escape, unescape};
use quick_xml::events::Event;
use quick_xml::Reader;

/// Parse an OPML document and return all feed URLs found in `xmlUrl` attributes.
///
/// Handles nested outlines (categories) at any depth.
/// Outlines without a `xmlUrl` attribute (e.g. category folders) are skipped.
pub fn parse_opml(xml: &str) -> Result<Vec<String>, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut urls = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                if e.name().as_ref().eq_ignore_ascii_case(b"outline") {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref().eq_ignore_ascii_case(b"xmlUrl") {
                            // attr.value is raw UTF-8 bytes (reader input is &str).
                            // Unescape XML entities (e.g. &amp; in query strings).
                            if let Ok(raw) = std::str::from_utf8(&attr.value) {
                                if let Ok(val) = unescape(raw) {
                                    let url = val.trim().to_string();
                                    if !url.is_empty() {
                                        urls.push(url);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!(
                "OPML parse error at position {}: {e}",
                reader.buffer_position()
            )),
            _ => {}
        }
        buf.clear();
    }

    Ok(urls)
}

/// Build an OPML 2.0 document from a list of `(url, title)` feed pairs.
///
/// Both `url` and `title` are XML-escaped so the output is always well-formed.
pub fn build_opml(feeds: &[(&str, &str)]) -> String {
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <opml version=\"2.0\">\n  \
           <head><title>Xaga Feeds</title></head>\n  \
           <body>\n",
    );
    for &(url, title) in feeds {
        let esc_url = escape(url);
        let esc_title = escape(title);
        s.push_str(&format!(
            "    <outline text=\"{esc_title}\" title=\"{esc_title}\" type=\"rss\" xmlUrl=\"{esc_url}\" />\n"
        ));
    }
    s.push_str("  </body>\n</opml>\n");
    s
}
