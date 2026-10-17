use quick_xml::escape::unescape;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

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
            Err(e) => {
                return Err(format!(
                    "OPML parse error at position {}: {e}",
                    reader.buffer_position()
                ));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(urls)
}

/// Build an OPML 2.0 document from a list of `(url, title)` feed pairs.
///
/// Element and attribute encoding is delegated to the XML writer.
pub fn build_opml(feeds: &[(&str, &str)]) -> String {
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    let mut write = |event| {
        writer
            .write_event(event)
            .expect("writing OPML to memory cannot fail");
    };

    write(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)));

    let mut opml = BytesStart::new("opml");
    opml.push_attribute(("version", "2.0"));
    write(Event::Start(opml));

    write(Event::Start(BytesStart::new("head")));
    write(Event::Start(BytesStart::new("title")));
    write(Event::Text(BytesText::new("Xaga Feeds")));
    write(Event::End(BytesEnd::new("title")));
    write(Event::End(BytesEnd::new("head")));

    write(Event::Start(BytesStart::new("body")));
    for &(url, title) in feeds {
        let mut outline = BytesStart::new("outline");
        outline.push_attribute(("text", title));
        outline.push_attribute(("title", title));
        outline.push_attribute(("type", "rss"));
        outline.push_attribute(("xmlUrl", url));
        write(Event::Empty(outline));
    }
    write(Event::End(BytesEnd::new("body")));
    write(Event::End(BytesEnd::new("opml")));

    String::from_utf8(writer.into_inner()).expect("OPML writer emits UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct ExportedOpml {
        version: String,
        title: String,
        outlines: Vec<(String, String, String, String)>,
    }

    fn parse_export(xml: &str) -> ExportedOpml {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut version = String::new();
        let mut title = String::new();
        let mut in_title = false;
        let mut outlines = Vec::new();

        loop {
            match reader.read_event_into(&mut buf).unwrap() {
                Event::Start(element) => match element.name().as_ref() {
                    b"opml" => {
                        version = attribute(&element, b"version");
                    }
                    b"title" => in_title = true,
                    _ => {}
                },
                Event::Empty(element) if element.name().as_ref() == b"outline" => {
                    outlines.push((
                        attribute(&element, b"text"),
                        attribute(&element, b"title"),
                        attribute(&element, b"type"),
                        attribute(&element, b"xmlUrl"),
                    ));
                }
                Event::Text(text) if in_title => {
                    title = unescape(std::str::from_utf8(text.as_ref()).unwrap())
                        .unwrap()
                        .into_owned();
                }
                Event::End(element) if element.name().as_ref() == b"title" => in_title = false,
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        ExportedOpml {
            version,
            title,
            outlines,
        }
    }

    fn attribute(element: &BytesStart<'_>, name: &[u8]) -> String {
        let value = element
            .attributes()
            .map(Result::unwrap)
            .find(|attribute| attribute.key.as_ref() == name)
            .unwrap()
            .value;
        unescape(std::str::from_utf8(&value).unwrap())
            .unwrap()
            .into_owned()
    }

    #[test]
    fn writes_empty_opml_document() {
        assert_eq!(
            parse_export(&build_opml(&[])),
            ExportedOpml {
                version: "2.0".into(),
                title: "Xaga Feeds".into(),
                outlines: vec![],
            }
        );
    }

    #[test]
    fn preserves_feed_order_and_attribute_values() {
        let feeds = [
            ("https://example.com/?a=1&b=\"two\"<x>", "A & \"B\" <C>"),
            ("https://例え.テスト/フィード", "日本語 📰"),
        ];

        assert_eq!(
            parse_export(&build_opml(&feeds)).outlines,
            vec![
                (
                    feeds[0].1.into(),
                    feeds[0].1.into(),
                    "rss".into(),
                    feeds[0].0.into(),
                ),
                (
                    feeds[1].1.into(),
                    feeds[1].1.into(),
                    "rss".into(),
                    feeds[1].0.into(),
                ),
            ]
        );
    }
}
