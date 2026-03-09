use super::EventType;

/// Convert a Matrix message type string to a WeChat EventType.
///
/// Mapping:
/// - "m.text" -> EventType::Text
/// - "m.image" -> EventType::Photo
/// - "m.audio" -> EventType::Audio
/// - "m.video" -> EventType::Video
/// - "m.file" -> EventType::File
/// - "m.location" -> EventType::Location
/// - other -> EventType::Text (default)
pub fn to_event_type(matrix_msgtype: &str) -> EventType {
    match matrix_msgtype {
        "m.text" => EventType::Text,
        "m.image" => EventType::Photo,
        "m.audio" => EventType::Audio,
        "m.video" => EventType::Video,
        "m.file" => EventType::File,
        "m.location" => EventType::Location,
        _ => EventType::Text,
    }
}

/// Convert a WeChat EventType to a Matrix message type string.
///
/// Mapping:
/// - EventType::Text -> "m.text"
/// - EventType::Photo -> "m.image"
/// - EventType::Sticker -> "m.image"
/// - EventType::Audio -> "m.audio"
/// - EventType::Video -> "m.video"
/// - EventType::File -> "m.file"
/// - EventType::Location -> "m.location"
/// - other -> "m.text" (default)
pub fn to_message_type(event_type: EventType) -> &'static str {
    match event_type {
        EventType::Text => "m.text",
        EventType::Photo => "m.image",
        EventType::Sticker => "m.image",
        EventType::Audio => "m.audio",
        EventType::Video => "m.video",
        EventType::File => "m.file",
        EventType::Location => "m.location",
        _ => "m.text",
    }
}
