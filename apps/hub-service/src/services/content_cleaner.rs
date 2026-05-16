/// 内容清洗管道 — 对转换后的 .md 进行安全清洗
/// 只做不会丢失信息的操作，高风险操作留给精炼阶段

/// 清洗配置
#[derive(Debug, Clone)]
pub struct CleanConfig {
    /// 压缩连续空行（3行→1行）
    pub compress_blank_lines: bool,
    /// 清除行尾空格
    pub trim_trailing_spaces: bool,
    /// 清除零宽字符
    pub strip_zero_width: bool,
    /// PDF中文字符间距修复
    pub fix_cjk_spacing: bool,
    /// PDF水印/重复行检测
    pub remove_watermarks: bool,
    /// PDF页眉页脚模板行检测
    pub remove_template_lines: bool,
    /// Word连续空格压缩
    pub compress_spaces: bool,
    /// Excel全空行过滤
    pub filter_empty_table_rows: bool,
    /// Session标题token统计清除
    pub strip_token_stats_from_title: bool,
    /// Session空session丢弃阈值(bytes)
    pub empty_session_threshold: usize,
    /// HEARTBEAT模板清除
    pub remove_heartbeat_template: bool,
    /// 相邻重复行去重
    pub dedup_adjacent_lines: bool,
    /// 超小文件阈值(bytes)
    pub tiny_file_threshold: usize,
}

impl Default for CleanConfig {
    fn default() -> Self {
        Self {
            compress_blank_lines: true,
            trim_trailing_spaces: true,
            strip_zero_width: true,
            fix_cjk_spacing: true,
            remove_watermarks: true,
            remove_template_lines: true,
            compress_spaces: true,
            filter_empty_table_rows: true,
            strip_token_stats_from_title: true,
            empty_session_threshold: 50,
            remove_heartbeat_template: true,
            dedup_adjacent_lines: true,
            tiny_file_threshold: 50,
        }
    }
}

/// 内容清洗器
pub struct ContentCleaner {
    config: CleanConfig,
}

impl ContentCleaner {
    pub fn new(config: CleanConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(CleanConfig::default())
    }

    // ========================================================================
    // 通用清洗 (B1-B3) — 所有文档类型都适用
    // ========================================================================

    /// B1: 压缩连续空行（3行以上 → 1个空行）
    pub fn compress_blank_lines(&self, input: &str) -> String {
        if !self.config.compress_blank_lines {
            return input.to_string();
        }
        let mut result = String::with_capacity(input.len());
        let mut blank_count = 0;
        for line in input.lines() {
            if line.trim().is_empty() {
                blank_count += 1;
                if blank_count <= 1 {
                    result.push('\n');
                }
            } else {
                blank_count = 0;
                result.push_str(line);
                result.push('\n');
            }
        }
        result
    }

    /// B2: 清除行尾空格
    pub fn trim_trailing_spaces(&self, input: &str) -> String {
        if !self.config.trim_trailing_spaces {
            return input.to_string();
        }
        input.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// B3: 清除零宽字符和不可见Unicode
    pub fn strip_zero_width(&self, input: &str) -> String {
        if !self.config.strip_zero_width {
            return input.to_string();
        }
        input.chars()
            .filter(|c| !matches!(c,
                '\u{200B}' |  // ZERO WIDTH SPACE
                '\u{200C}' |  // ZERO WIDTH NON-JOINER
                '\u{200D}' |  // ZERO WIDTH JOINER
                '\u{FEFF}' |  // BOM / ZERO WIDTH NO-BREAK SPACE
                '\u{200E}' |  // LEFT-TO-RIGHT MARK
                '\u{200F}' |  // RIGHT-TO-LEFT MARK
                '\u{202A}' |  // LEFT-TO-RIGHT EMBEDDING
                '\u{202B}' |  // RIGHT-TO-LEFT EMBEDDING
                '\u{202C}' |  // POP DIRECTIONAL FORMATTING
                '\u{202D}' |  // LEFT-TO-RIGHT OVERRIDE
                '\u{202E}' |  // RIGHT-TO-LEFT OVERRIDE
                '\u{2060}' |  // WORD JOINER
                '\u{2061}' |  // FUNCTION APPLICATION
                '\u{2062}' |  // INVISIBLE TIMES
                '\u{2063}' |  // INVISIBLE SEPARATOR
                '\u{2064}'    // INVISIBLE PLUS
            ))
            .collect()
    }

    /// B10: 相邻重复行去重
    pub fn dedup_adjacent_lines(&self, input: &str) -> String {
        if !self.config.dedup_adjacent_lines {
            return input.to_string();
        }
        let mut result = String::with_capacity(input.len());
        let mut prev_line = "";
        for line in input.lines() {
            let trimmed = line.trim();
            // 只对非空、非短行去重（避免误伤分隔符等）
            if trimmed.is_empty() || trimmed.len() < 5 || trimmed != prev_line.trim() {
                result.push_str(line);
                result.push('\n');
            }
            prev_line = line;
        }
        result
    }

    // ========================================================================
    // PDF清洗 (A1-A4)
    // ========================================================================

    /// A1: 修复中文字符之间的异常空格
    /// "阿 里 巴 巴" → "阿里巴巴"
    pub fn fix_cjk_spacing(&self, input: &str) -> String {
        if !self.config.fix_cjk_spacing {
            return input.to_string();
        }
        let mut result = String::with_capacity(input.len());
        let chars: Vec<char> = input.chars().collect();
        let len = chars.len();

        let mut i = 0;
        while i < len {
            if i + 2 < len
                && is_cjk(chars[i])
                && chars[i + 1] == ' '
                && is_cjk(chars[i + 2])
            {
                // CJK + space + CJK → 去掉空格
                result.push(chars[i]);
                result.push(chars[i + 2]);
                i += 3;
                // 继续检查下一个模式: "阿 里 巴 巴" 连续匹配
                while i + 1 < len && chars[i] == ' ' && is_cjk(chars[i + 1]) {
                    result.push(chars[i + 1]);
                    i += 2;
                }
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }
        result
    }

    /// A3: 检测并移除水印（重复出现≥3次的相同短行）
    pub fn remove_watermarks(&self, input: &str) -> String {
        if !self.config.remove_watermarks {
            return input.to_string();
        }

        // 统计每行出现次数（只统计短行，<80字符）
        let mut line_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for line in input.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && trimmed.len() < 80 {
                *line_counts.entry(trimmed.to_string()).or_insert(0) += 1;
            }
        }

        // 找出重复≥3次的行作为水印
        let watermarks: std::collections::HashSet<String> = line_counts.iter()
            .filter(|(_, &count)| count >= 3)
            .map(|(line, _)| line.clone())
            .collect();

        if watermarks.is_empty() {
            return input.to_string();
        }

        // 过滤水印行
        input.lines()
            .filter(|line| !watermarks.contains(line.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A4: 移除PDF模板行（标签+冒号+空白的模式）
    /// 例如: "文 件 编 号 ： 页 数 ： 8页"
    ///       "适 用 部 门 ： 实 施 日 期 ："
    pub fn remove_template_lines(&self, input: &str) -> String {
        if !self.config.remove_template_lines {
            return input.to_string();
        }
        input.lines()
            .filter(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return true; // 保留空行
                }
                // 检测模式: 只含中文标签+冒号/空格，没有实际内容
                // "编 制 人 ：  编 制 时 间 ：" → 移除
                // "确认日期： 确认日期： 确认日期：" → 移除
                let content_after_colons: String = trimmed.chars()
                    .collect::<Vec<_>>()
                    .windows(1)
                    .filter_map(|w| {
                        // 跳过冒号前的内容
                        None::<char>
                    })
                    .collect();

                // 简化判断: 如果行内有≥2个"："且冒号后都是空白，判定为模板行
                let colon_count = trimmed.matches('：').count()
                    + trimmed.matches(':').count();
                if colon_count >= 2 {
                    // 检查每个冒号后面是否都是空白或另一个标签
                    let parts: Vec<&str> = trimmed.split(|c| c == '：' || c == ':').collect();
                    let mostly_empty = parts.iter().skip(1).all(|p| {
                        let t = p.trim();
                        t.is_empty() || t.chars().all(|c| c.is_whitespace() || is_cjk(c))
                    });
                    if mostly_empty {
                        return false; // 移除
                    }
                }
                true // 保留
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    // ========================================================================
    // Word清洗 (A7)
    // ========================================================================

    /// A7: 压缩连续空格（保留行首缩进）
    pub fn compress_spaces(&self, input: &str) -> String {
        if !self.config.compress_spaces {
            return input.to_string();
        }
        input.lines()
            .map(|line| {
                // 保留行首空格（最多4个），压缩其余连续空格
                let leading = line.len() - line.trim_start().len();
                let leading_spaces: String = line.chars().take(leading).collect();
                let rest = &line[leading..];
                let compressed: String = rest.split_whitespace().collect::<Vec<_>>().join(" ");
                if leading_spaces.is_empty() {
                    compressed
                } else {
                    format!("{}{}", leading_spaces, compressed)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    // ========================================================================
    // Excel清洗 (A9)
    // ========================================================================

    /// A9: 过滤Markdown表格中的全空行
    pub fn filter_empty_table_rows(&self, input: &str) -> String {
        if !self.config.filter_empty_table_rows {
            return input.to_string();
        }
        input.lines()
            .filter(|line| {
                let trimmed = line.trim();
                // 保留非表格行
                if !trimmed.starts_with('|') {
                    return true;
                }
                // 保留表头分隔行 | --- | --- |
                if trimmed.contains("---") {
                    return true;
                }
                // 检查是否全空: 去掉 | 和空格后是否还有内容
                let content: String = trimmed.chars()
                    .filter(|c| *c != '|' && !c.is_whitespace())
                    .collect();
                !content.is_empty()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    // ========================================================================
    // Session清洗 (B7-B10)
    // ========================================================================

    /// B7: 从标题中移除token统计
    /// "(11934612in/28476out tokens)" → ""
    pub fn strip_token_stats_from_title(&self, title: &str) -> String {
        if !self.config.strip_token_stats_from_title {
            return title.to_string();
        }
        // 匹配 (数字in/数字out tokens) 模式
        let mut result = title.to_string();
        while let Some(start) = result.rfind('(') {
            let rest = &result[start..];
            if rest.contains("in/") && rest.contains("out tokens)") {
                if let Some(end) = rest.find(')') {
                    result = format!("{}{}", &result[..start], &result[start + end + 1..]);
                    continue;
                }
            }
            break;
        }
        result.trim().to_string()
    }

    /// B9: 清除HEARTBEAT模板
    pub fn remove_heartbeat_template(&self, input: &str) -> String {
        if !self.config.remove_heartbeat_template {
            return input.to_string();
        }
        // HEARTBEAT模板特征: 以 "Read HEARTBEAT.md" 开头的段落
        let lines: Vec<&str> = input.lines().collect();
        let mut result = Vec::with_capacity(lines.len());
        let mut skip_block = false;

        for line in &lines {
            let trimmed = line.trim();
            // 检测HEARTBEAT模板开始
            if trimmed.starts_with("Read HEARTBEAT.md")
                || trimmed.starts_with("Follow it strictly")
                || trimmed.starts_with("Current time:")
                || (trimmed.starts_with("When reading HEARTBEAT.md") && trimmed.contains("workspace"))
            {
                skip_block = true;
                continue;
            }
            // 检测HEARTBEAT模板结束（遇到非模板内容）
            if skip_block {
                if trimmed.is_empty() {
                    continue; // 跳过模板后的空行
                }
                if !trimmed.starts_with("If nothing needs attention")
                    && !trimmed.starts_with("reply HEARTBEAT_OK")
                    && !trimmed.starts_with("Do not infer")
                    && !trimmed.starts_with("Do not read")
                {
                    skip_block = false;
                    result.push(*line);
                }
                continue;
            }
            result.push(*line);
        }
        result.join("\n")
    }

    // ========================================================================
    // 组合清洗管道
    // ========================================================================

    /// 清洗知识库文档（完整管道）
    pub fn clean_knowledge(&self, content: &str) -> String {
        let mut text = content.to_string();

        // PDF相关清洗
        text = self.fix_cjk_spacing(&text);
        text = self.remove_watermarks(&text);
        text = self.remove_template_lines(&text);

        // 通用清洗
        text = self.compress_spaces(&text);
        text = self.filter_empty_table_rows(&text);
        text = self.trim_trailing_spaces(&text);
        text = self.strip_zero_width(&text);
        text = self.compress_blank_lines(&text);
        text = self.dedup_adjacent_lines(&text);

        text
    }

    /// 清洗Memory条目
    pub fn clean_memory(&self, content: &str) -> String {
        let mut text = content.to_string();
        text = self.trim_trailing_spaces(&text);
        text = self.strip_zero_width(&text);
        text = self.compress_blank_lines(&text);
        text
    }

    /// 清洗Session内容
    pub fn clean_session(&self, content: &str) -> String {
        let mut text = content.to_string();
        text = self.remove_heartbeat_template(&text);
        text = self.dedup_adjacent_lines(&text);
        text = self.trim_trailing_spaces(&text);
        text = self.strip_zero_width(&text);
        text = self.compress_blank_lines(&text);
        text
    }

    /// 清洗Session标题
    pub fn clean_session_title(&self, title: &str) -> String {
        let mut text = title.to_string();
        text = self.strip_token_stats_from_title(&text);
        text
    }

    /// 检查文档是否太小（应丢弃或标记）
    pub fn is_tiny(&self, content: &str) -> bool {
        content.trim().len() < self.config.tiny_file_threshold
    }

    /// 检查Session是否为空（只有用户消息没有回复）
    pub fn is_empty_session(&self, content: &str) -> bool {
        if content.len() > self.config.empty_session_threshold {
            return false;
        }
        !content.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("assistant:")
                || trimmed.starts_with("gemini:")
                || trimmed.starts_with("codex:")
                || trimmed.starts_with("openclaw:")
        })
    }
}

/// 判断是否为CJK字符
fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |   // CJK Unified Ideographs
        '\u{3400}'..='\u{4DBF}' |   // CJK Unified Ideographs Extension A
        '\u{F900}'..='\u{FAFF}' |   // CJK Compatibility Ideographs
        '\u{2E80}'..='\u{2EFF}' |   // CJK Radicals Supplement
        '\u{3000}'..='\u{303F}' |   // CJK Symbols and Punctuation
        '\u{FF00}'..='\u{FFEF}'     // Fullwidth Forms
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_blank_lines() {
        let cleaner = ContentCleaner::with_defaults();
        let input = "line1\n\n\n\nline2\n\n\nline3";
        let result = cleaner.compress_blank_lines(input);
        assert_eq!(result, "line1\n\nline2\n\nline3\n");
    }

    #[test]
    fn test_fix_cjk_spacing() {
        let cleaner = ContentCleaner::with_defaults();
        let input = "阿 里 巴 巴 价 值 观 考 核 体 系";
        let result = cleaner.fix_cjk_spacing(input);
        assert_eq!(result, "阿里巴巴价值观考核体系");
    }

    #[test]
    fn test_fix_cjk_spacing_preserves_normal() {
        let cleaner = ContentCleaner::with_defaults();
        let input = "Hello World 你好世界";
        let result = cleaner.fix_cjk_spacing(input);
        assert_eq!(result, "Hello World 你好世界");
    }

    #[test]
    fn test_strip_zero_width() {
        let cleaner = ContentCleaner::with_defaults();
        let input = "Hello\u{200B}World\u{FEFF}";
        let result = cleaner.strip_zero_width(input);
        assert_eq!(result, "HelloWorld");
    }

    #[test]
    fn test_filter_empty_table_rows() {
        let cleaner = ContentCleaner::with_defaults();
        let input = "| A | B |\n| --- | --- |\n| | |\n| 1 | 2 |\n| | |";
        let result = cleaner.filter_empty_table_rows(input);
        assert_eq!(result, "| A | B |\n| --- | --- |\n| 1 | 2 |");
    }

    #[test]
    fn test_strip_token_stats() {
        let cleaner = ContentCleaner::with_defaults();
        let title = "Gemini Session session-2026-05-05 [gemini-3-flash-preview] (11934612in/28476out tokens)";
        let result = cleaner.strip_token_stats_from_title(title);
        assert_eq!(result, "Gemini Session session-2026-05-05 [gemini-3-flash-preview]");
    }

    #[test]
    fn test_dedup_adjacent_lines() {
        let cleaner = ContentCleaner::with_defaults();
        let input = "hello world\nhello world\ndifferent line";
        let result = cleaner.dedup_adjacent_lines(input);
        assert_eq!(result, "hello world\ndifferent line\n");
    }

    #[test]
    fn test_is_empty_session() {
        let cleaner = ContentCleaner::with_defaults();
        assert!(cleaner.is_empty_session("user: hello"));
        assert!(!cleaner.is_empty_session("user: hello\nassistant: hi there"));
    }

    #[test]
    fn test_remove_watermarks() {
        let cleaner = ContentCleaner::with_defaults();
        let input = "关注公众号：HR成长指南\n正文1\n关注公众号：HR成长指南\n正文2\n关注公众号：HR成长指南";
        let result = cleaner.remove_watermarks(input);
        assert!(!result.contains("关注公众号"));
        assert!(result.contains("正文1"));
    }
}
