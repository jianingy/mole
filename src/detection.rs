// Jianing Yang <jianingy.yang@gmail.com> @ 22 Sep, 2016
use regex::Regex;
use std::io::Result as IoResult;
use std::vec;

lazy_static! {
    static ref RE_DOC_DELIMITER: Regex  = Regex::new("--+\n").unwrap();
}
pub fn rules() -> IoResult<vec::IntoIter<(String, String, String)>> {
    let text = include_str!("detection.inc");
    let docs = RE_DOC_DELIMITER.split(text);
    let docs = docs.map(|doc| {
            let mut parts = doc.split("\n\n");
            match (parts.next(), parts.next(), parts.next()) {
                (Some(x), Some(y), Some(z)) => {
                    let y = y.replace("\\r", "\r").replace("\\n", "\n");
                    let z = z.trim().to_string();
                    Some((x.to_string(), y, z))
                },
                _ => None,
            }
        })
        .filter(|x| x.is_some())
        .map(|x| x.unwrap())
        .collect::<Vec<(String, String, String)>>();

    Ok(docs.into_iter())
}
