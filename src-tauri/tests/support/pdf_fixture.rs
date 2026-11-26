pub(super) fn pdf_fixture() -> Vec<u8> {
    use std::io::Write;
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0];
    for (index, object) in [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>",
    ]
    .iter()
    .enumerate()
    {
        offsets.push(bytes.len());
        write!(bytes, "{} 0 obj\n{object}\nendobj\n", index + 1).unwrap();
    }
    let xref = bytes.len();
    write!(bytes, "xref\n0 4\n0000000000 65535 f \n").unwrap();
    for offset in offsets.iter().skip(1) {
        writeln!(bytes, "{offset:010} 00000 n ").unwrap();
    }
    write!(
        bytes,
        "trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    )
    .unwrap();
    bytes
}
