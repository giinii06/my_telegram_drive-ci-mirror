use std::fmt::Write as _;
use tdrive_storage::FilePart;

pub fn encode_json(parts: &[FilePart]) -> String {
    let mut output = String::from("{\"parts\":[");
    for (index, part) in parts.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(
            &mut output,
            "{{\"partIndex\":\"{}\",\"msgId\":\"{}\",\"size\":\"{}\"}}",
            part.part_index, part.msg_id, part.size
        )
        .expect("String writes cannot fail");
    }
    output.push_str("]}");
    output
}
