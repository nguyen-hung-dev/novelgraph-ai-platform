use crate::{ChapterPreview, ImportPreview, NovelImportInput};

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
