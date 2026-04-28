use crate::{ChapterPreview, DraftExtractionPrompt, ImportPreview, NovelImportInput};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterDraft {
    pub chapter_num: i64,
    pub title: String,
    pub start_char: usize,
    pub end_char: usize,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSegmentDraft {
    pub segment_index: i64,
    pub start_char: usize,
    pub end_char: usize,
    pub text: String,
}

pub fn build_import_preview(input: &NovelImportInput) -> ImportPreview {
    let chapters = split_chapters(&input.text);

    ImportPreview {
        title: input.title.trim().to_string(),
        total_chars: input.text.chars().count(),
        chapter_count: chapters.len(),
        chapters: chapters
            .into_iter()
            .map(|chapter| ChapterPreview {
                chapter_num: chapter.chapter_num,
                title: chapter.title,
                start_char: chapter.start_char,
                end_char: chapter.end_char,
                char_count: chapter.end_char.saturating_sub(chapter.start_char),
                preview: preview_text(&chapter.content, 180),
            })
            .collect(),
    }
}

pub fn detect_basic_source_language(source: &str) -> Option<String> {
    let mut han_count = 0usize;
    let mut kana_count = 0usize;
    let mut hangul_count = 0usize;
    let mut latin_count = 0usize;
    let mut vietnamese_mark_count = 0usize;

    for ch in source.chars().take(12_000) {
        if is_han_char(ch) {
            han_count += 1;
        } else if is_kana_char(ch) {
            kana_count += 1;
        } else if is_hangul_char(ch) {
            hangul_count += 1;
        } else if ch.is_ascii_alphabetic() {
            latin_count += 1;
        } else if is_vietnamese_marked_char(ch) {
            latin_count += 1;
            vietnamese_mark_count += 1;
        }
    }

    if kana_count >= 8 {
        return Some("ja".to_string());
    }
    if hangul_count >= 8 {
        return Some("ko".to_string());
    }
    if han_count >= 16 && han_count > latin_count {
        return Some("zh".to_string());
    }
    if vietnamese_mark_count >= 8 {
        return Some("vi".to_string());
    }
    if latin_count >= 32 {
        return Some("en".to_string());
    }

    None
}

pub fn build_novel_metadata_suggestion_prompt(input: &NovelImportInput) -> DraftExtractionPrompt {
    let source_language = input
        .source_language
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "auto")
        .map(str::to_string)
        .or_else(|| detect_basic_source_language(&input.text))
        .unwrap_or_else(|| "unknown".to_string());
    let sample = preview_text(&input.text, 12_000);

    DraftExtractionPrompt {
        schema_version: "novel_metadata_suggestion.v1",
        system_prompt: "You extract novel metadata from source text. Return valid JSON only."
            .to_string(),
        user_prompt: format!(
            r#"Source language guess: {source_language}
Existing title: {title}
Existing author: {author}
Existing genre: {genre}
Existing description: {description}

Source sample:
<<<SOURCE_TEXT
{sample}
SOURCE_TEXT

Trích xuất metadata cấp truyện từ sample. Chỉ dùng thông tin có trong sample; không dùng kiến thức ngoài.
Nếu không chắc trường nào, trả null cho trường đó.
source_language dùng ISO code ngắn như zh, vi, en, ja, ko nếu nhận diện được.
genre là chuỗi ngắn, có thể gồm nhiều thể loại cách nhau bằng dấu phẩy nếu sample thể hiện rõ.
description là mô tả rất ngắn bằng tiếng Việt có dấu về nội dung/bối cảnh nếu suy ra được từ sample, không bịa chi tiết.

Chỉ trả JSON array đúng một object:
[
  {{
    "title": null,
    "author": null,
    "source_language": "{source_language}",
    "genre": null,
    "description": null,
    "confidence": 0.0
  }}
]"#,
            source_language = source_language,
            title = input.title.trim(),
            author = input.author.as_deref().unwrap_or("").trim(),
            genre = input.genre.as_deref().unwrap_or("").trim(),
            description = input.description.as_deref().unwrap_or("").trim(),
            sample = sample
        ),
    }
}

pub fn split_chapters(source: &str) -> Vec<ChapterDraft> {
    let total_chars = source.chars().count();
    if source.trim().is_empty() {
        return Vec::new();
    }

    let mut headings = Vec::new();
    let mut cursor = 0usize;

    for line in source.split_inclusive('\n') {
        let trimmed = line.trim();
        if let Some(title) = chapter_heading_title(trimmed) {
            headings.push((cursor, title));
        }
        cursor += line.chars().count();
    }

    if headings.is_empty() {
        return vec![ChapterDraft {
            chapter_num: 1,
            title: "Chapter 1".to_string(),
            start_char: 0,
            end_char: total_chars,
            content: source.to_string(),
        }];
    }

    if headings[0].0 > 0 {
        let prefix = slice_chars(source, 0, headings[0].0);
        if !prefix.trim().is_empty() {
            headings.insert(0, (0, "Preface".to_string()));
        }
    }

    headings
        .iter()
        .enumerate()
        .filter_map(|(index, (start_char, title))| {
            let end_char = headings
                .get(index + 1)
                .map(|(next_start, _)| *next_start)
                .unwrap_or(total_chars);
            let content = slice_chars(source, *start_char, end_char);
            if content.trim().is_empty() {
                return None;
            }

            Some(ChapterDraft {
                chapter_num: index as i64 + 1,
                title: title.clone(),
                start_char: *start_char,
                end_char,
                content,
            })
        })
        .collect()
}

pub fn split_source_segments(chapter_content: &str) -> Vec<SourceSegmentDraft> {
    let mut segments = Vec::new();
    let mut segment_start = None;
    let mut cursor = 0usize;

    for line in chapter_content.split_inclusive('\n') {
        let line_len = line.chars().count();
        if line.trim().is_empty() {
            if let Some(start_char) = segment_start.take() {
                push_segment(&mut segments, chapter_content, start_char, cursor);
            }
        } else if segment_start.is_none() {
            segment_start = Some(cursor);
        }
        cursor += line_len;
    }

    if let Some(start_char) = segment_start {
        push_segment(
            &mut segments,
            chapter_content,
            start_char,
            chapter_content.chars().count(),
        );
    }

    if segments.is_empty() && !chapter_content.trim().is_empty() {
        push_segment(
            &mut segments,
            chapter_content,
            0,
            chapter_content.chars().count(),
        );
    }

    segments
}

fn push_segment(
    segments: &mut Vec<SourceSegmentDraft>,
    chapter_content: &str,
    start_char: usize,
    end_char: usize,
) {
    let text = slice_chars(chapter_content, start_char, end_char);
    if text.trim().is_empty() {
        return;
    }

    segments.push(SourceSegmentDraft {
        segment_index: segments.len() as i64 + 1,
        start_char,
        end_char,
        text,
    });
}

fn chapter_heading_title(trimmed: &str) -> Option<String> {
    let char_count = trimmed.chars().count();
    if trimmed.is_empty() || char_count > 96 {
        return None;
    }

    let markdown_title = trimmed
        .starts_with('#')
        .then(|| trimmed.trim_start_matches('#').trim())
        .filter(|title| !title.is_empty());
    let candidate = markdown_title.unwrap_or(trimmed);
    let lower = candidate.to_lowercase();
    let is_known_heading = lower == "prologue"
        || lower == "epilogue"
        || lower.starts_with("chapter ")
        || lower.starts_with("chapter\t")
        || lower.starts_with("chapter:")
        || lower.starts_with("chap. ")
        || lower.starts_with("chương ")
        || lower.starts_with("chương\t")
        || lower.starts_with("chương:")
        || lower.starts_with("chuong ")
        || lower.starts_with("chuong:")
        || (candidate.starts_with('第') && candidate.contains('章') && char_count <= 40);

    if markdown_title.is_some() || is_known_heading {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn preview_text(source: &str, max_chars: usize) -> String {
    let normalized = source.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }

    normalized.chars().take(max_chars).collect::<String>()
}

fn is_han_char(ch: char) -> bool {
    matches!(ch as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF)
}

fn is_kana_char(ch: char) -> bool {
    matches!(ch as u32, 0x3040..=0x30FF)
}

fn is_hangul_char(ch: char) -> bool {
    matches!(ch as u32, 0xAC00..=0xD7AF | 0x1100..=0x11FF)
}

fn is_vietnamese_marked_char(ch: char) -> bool {
    matches!(
        ch,
        'à' | 'á'
            | 'ả'
            | 'ã'
            | 'ạ'
            | 'ă'
            | 'ằ'
            | 'ắ'
            | 'ẳ'
            | 'ẵ'
            | 'ặ'
            | 'â'
            | 'ầ'
            | 'ấ'
            | 'ẩ'
            | 'ẫ'
            | 'ậ'
            | 'è'
            | 'é'
            | 'ẻ'
            | 'ẽ'
            | 'ẹ'
            | 'ê'
            | 'ề'
            | 'ế'
            | 'ể'
            | 'ễ'
            | 'ệ'
            | 'ì'
            | 'í'
            | 'ỉ'
            | 'ĩ'
            | 'ị'
            | 'ò'
            | 'ó'
            | 'ỏ'
            | 'õ'
            | 'ọ'
            | 'ô'
            | 'ồ'
            | 'ố'
            | 'ổ'
            | 'ỗ'
            | 'ộ'
            | 'ơ'
            | 'ờ'
            | 'ớ'
            | 'ở'
            | 'ỡ'
            | 'ợ'
            | 'ù'
            | 'ú'
            | 'ủ'
            | 'ũ'
            | 'ụ'
            | 'ư'
            | 'ừ'
            | 'ứ'
            | 'ử'
            | 'ữ'
            | 'ự'
            | 'ỳ'
            | 'ý'
            | 'ỷ'
            | 'ỹ'
            | 'ỵ'
            | 'đ'
            | 'À'
            | 'Á'
            | 'Ả'
            | 'Ã'
            | 'Ạ'
            | 'Ă'
            | 'Ằ'
            | 'Ắ'
            | 'Ẳ'
            | 'Ẵ'
            | 'Ặ'
            | 'Â'
            | 'Ầ'
            | 'Ấ'
            | 'Ẩ'
            | 'Ẫ'
            | 'Ậ'
            | 'È'
            | 'É'
            | 'Ẻ'
            | 'Ẽ'
            | 'Ẹ'
            | 'Ê'
            | 'Ề'
            | 'Ế'
            | 'Ể'
            | 'Ễ'
            | 'Ệ'
            | 'Ì'
            | 'Í'
            | 'Ỉ'
            | 'Ĩ'
            | 'Ị'
            | 'Ò'
            | 'Ó'
            | 'Ỏ'
            | 'Õ'
            | 'Ọ'
            | 'Ô'
            | 'Ồ'
            | 'Ố'
            | 'Ổ'
            | 'Ỗ'
            | 'Ộ'
            | 'Ơ'
            | 'Ờ'
            | 'Ớ'
            | 'Ở'
            | 'Ỡ'
            | 'Ợ'
            | 'Ù'
            | 'Ú'
            | 'Ủ'
            | 'Ũ'
            | 'Ụ'
            | 'Ư'
            | 'Ừ'
            | 'Ứ'
            | 'Ử'
            | 'Ữ'
            | 'Ự'
            | 'Ỳ'
            | 'Ý'
            | 'Ỷ'
            | 'Ỹ'
            | 'Ỵ'
            | 'Đ'
    )
}

fn slice_chars(source: &str, start: usize, end: usize) -> String {
    source
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{split_chapters, split_source_segments};

    #[test]
    fn splits_vietnamese_chapter_headings() {
        let source = "Lời mở đầu\n\nChương 1\nMột khởi đầu.\n\nChương 2\nTiếp tục.";
        let chapters = split_chapters(source);

        assert_eq!(chapters.len(), 3);
        assert_eq!(chapters[0].title, "Preface");
        assert_eq!(chapters[1].title, "Chương 1");
        assert_eq!(chapters[2].title, "Chương 2");
    }

    #[test]
    fn splits_english_chinese_and_markdown_headings() {
        let english = split_chapters("Chapter 1\nStart.\n\nChapter 2\nContinue.");
        let chinese = split_chapters("第1章 初见\n开始。\n\n第2章 再会\n继续。");
        let markdown = split_chapters("# Opening\nStart.\n\n## Second Part\nContinue.");

        assert_eq!(english.len(), 2);
        assert_eq!(chinese.len(), 2);
        assert_eq!(markdown.len(), 2);
        assert_eq!(markdown[0].title, "Opening");
        assert_eq!(markdown[1].title, "Second Part");
    }

    #[test]
    fn falls_back_to_single_chapter_without_headings() {
        let chapters = split_chapters("Một đoạn truyện không có tiêu đề chương.");

        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, "Chapter 1");
    }

    #[test]
    fn splits_source_segments_by_blank_lines() {
        let segments = split_source_segments("Chương 1\nDòng một.\n\nDòng hai.");

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].segment_index, 1);
        assert_eq!(segments[1].segment_index, 2);
    }
}
