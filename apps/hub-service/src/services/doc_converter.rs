use std::path::{Path, PathBuf};

/// 文档转换工具：docx/xlsx/pptx → .md
/// 转换结果存放在软件缓存目录，目录结构与原始一致
pub struct DocConverter {
    cache_dir: PathBuf,
}

impl DocConverter {
    pub fn new(cache_dir: PathBuf) -> Self {
        let doc_cache = cache_dir.join("doc_convert");
        Self { cache_dir: doc_cache }
    }
    
    /// 检查是否需要转换（仅二进制/复杂格式，纯文本格式直接使用无需转换）
    pub fn needs_conversion(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            matches!(ext.as_str(),
                // 二进制文档格式（需要实际转换）
                "docx" | "doc" | "xlsx" | "xls" | "pptx" | "ppt" |
                "odt" | "ods" | "odp" | "pdf" | "rtf" |
                "html" | "htm" | "mhtml" | "mht" | "epub"
            )
        } else {
            false
        }
    }
    
    /// 获取缓存路径，保持目录结构一致
    /// 例如: D:\docs\work\report.docx → cache_dir/D/docs/work/report.md
    pub fn get_cache_path(&self, source: &Path) -> PathBuf {
        // 获取盘符（Windows）或根路径
        let _components: Vec<_> = source.components().collect();
        
        // 构建相对路径
        let relative = if source.to_string_lossy().contains(':') {
            // Windows 路径：去掉盘符如 "C:"
            let mut path_str = source.to_string_lossy().to_string();
            // 去掉 "C:" 或 "D:" 等
            if let Some(pos) = path_str.find(':') {
                path_str = path_str[pos+1..].to_string();
            }
            // 去掉开头的 \
            path_str.trim_start_matches('\\').trim_start_matches('/').to_string()
        } else {
            source.to_string_lossy().to_string()
        };
        
        // 把扩展名改为 .md
        let md_relative = Path::new(&relative).with_extension("md");
        
        self.cache_dir.join(md_relative)
    }
    
    /// 检查是否需要重新转换
    pub fn needs_update(&self, source: &Path) -> bool {
        let target = self.get_cache_path(source);
        
        if !target.exists() {
            return true;
        }
        
        let source_meta = std::fs::metadata(source).ok();
        let target_meta = std::fs::metadata(target).ok();
        
        match (source_meta, target_meta) {
            (Some(s), Some(t)) => {
                let source_time = s.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                let target_time = t.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                source_time > target_time
            }
            _ => true,
        }
    }
    
    /// 转换文档
    pub fn convert(&self, source: &Path) -> Result<PathBuf, String> {
        let target = self.get_cache_path(source);
        
        // 确保目录存在
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create cache dir: {}", e))?;
        }
        
        // 检查是否需要更新
        if !self.needs_update(source) {
            return Ok(target);
        }
        
        let ext = source.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        let content = match ext.as_str() {
            // Office 文档
            "docx" => Self::convert_docx(source)?,
            "doc" => Self::convert_doc_legacy(source)?,
            "xlsx" | "xls" => Self::convert_xlsx(source)?,
            "pptx" => Self::convert_pptx(source)?,
            "ppt" => Self::convert_ppt_legacy(source)?,
            // OpenDocument
            "odt" => Self::convert_odt(source)?,
            "ods" => Self::convert_ods(source)?,
            "odp" => Self::convert_odp(source)?,
            // PDF
            "pdf" => Self::convert_pdf(source)?,
            // RTF
            "rtf" => Self::convert_rtf(source)?,
            // HTML
            "html" | "htm" => Self::convert_html(source)?,
            "mhtml" | "mht" => Self::convert_html(source)?,
            // EPUB
            "epub" => Self::convert_epub(source)?,
            _ => return Err(format!("Unsupported file type: {}", ext)),
        };
        
        std::fs::write(&target, content)
            .map_err(|e| format!("Failed to write cache: {}", e))?;
        
        Ok(target)
    }
    
    /// 获取或转换（缓存有效则直接返回）
    pub fn get_or_convert(&self, source: &Path) -> Result<PathBuf, String> {
        if !self.needs_update(source) {
            let cached = self.get_cache_path(source);
            if cached.exists() {
                return Ok(cached);
            }
        }
        self.convert(source)
    }
    
    /// 清理缓存
    #[allow(dead_code)]
    pub fn clear_cache(&self) -> Result<(), String> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)
                .map_err(|e| format!("Failed to clear cache: {}", e))?;
        }
        Ok(())
    }
    
    fn convert_docx(path: &Path) -> Result<String, String> {
        use docx_rs::read_docx;
        
        let buf = std::fs::read(path)
            .map_err(|e| format!("Failed to read: {}", e))?;
        
        // Wrap in catch_unwind because docx-rs can panic on malformed files
        let docx = std::panic::catch_unwind(|| read_docx(&buf))
            .map_err(|_| "docx-rs panicked parsing this file (malformed docx)".to_string())?
            .map_err(|e| format!("Failed to parse: {}", e))?;
        
        let mut md = String::new();
        
        for child in docx.document.children {
            if let docx_rs::DocumentChild::Paragraph(para) = child {
                let mut text = String::new();
                let mut is_heading = false;
                let mut heading_level = 0;
                
                if let Some(style) = &para.property.style {
                    if style.val.starts_with("Heading") {
                        is_heading = true;
                        heading_level = style.val.replace("Heading", "").parse().unwrap_or(1);
                    }
                }
                
                for content in &para.children {
                    if let docx_rs::ParagraphChild::Run(run) = content {
                        for run_child in &run.children {
                            if let docx_rs::RunChild::Text(t) = run_child {
                                text.push_str(&t.text);
                            }
                        }
                    }
                }
                
                if text.trim().is_empty() {
                    continue;
                }
                
                if is_heading {
                    md.push_str(&format!("{} {}\n\n", "#".repeat(heading_level.min(6)), text.trim()));
                } else {
                    md.push_str(&format!("{}\n\n", text.trim()));
                }
            }
        }
        
        Ok(md)
    }
    
    fn convert_xlsx(path: &Path) -> Result<String, String> {
        use calamine::{Reader, open_workbook, Xlsx, Data};
        
        let mut workbook: Xlsx<_> = open_workbook(path)
            .map_err(|e| format!("Failed to open: {}", e))?;
        
        let mut md = String::new();
        
        for sheet_name in workbook.sheet_names().to_owned() {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                md.push_str(&format!("## {}\n\n", sheet_name));
                
                let mut rows: Vec<Vec<String>> = Vec::new();
                for row in range.rows() {
                    rows.push(row.iter().map(|c| match c {
                        Data::String(s) => s.clone(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        _ => String::new(),
                    }).collect());
                }
                
                if let Some(header) = rows.first() {
                    md.push_str(&format!("| {} |\n", header.join(" | ")));
                    md.push_str(&format!("| {} |\n", header.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")));
                }
                for row in rows.iter().skip(1) {
                    md.push_str(&format!("| {} |\n", row.join(" | ")));
                }
                md.push('\n');
            }
        }
        
        Ok(md)
    }
    
    fn convert_pptx(path: &Path) -> Result<String, String> {
        use zip::ZipArchive;
        use quick_xml::Reader;
        use quick_xml::events::Event;
        use quick_xml::name::QName;
        
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
        
        let mut slide_files: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
            .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
            .collect();
        slide_files.sort();
        
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
        
        let mut md = String::new();
        
        for (idx, slide_name) in slide_files.iter().enumerate() {
            if let Ok(mut file) = archive.by_name(slide_name) {
                let mut xml = String::new();
                std::io::Read::read_to_string(&mut file, &mut xml).ok();
                
                let mut reader = Reader::from_str(&xml);
                let mut texts = Vec::new();
                let mut current = String::new();
                let mut in_text = false;
                
                loop {
                    match reader.read_event() {
                        Ok(Event::Start(ref e)) if e.name() == QName(b"a:t") => in_text = true,
                        Ok(Event::Text(ref e)) if in_text => {
                            if let Ok(t) = e.unescape() { current.push_str(&t); }
                        }
                        Ok(Event::End(ref e)) if e.name() == QName(b"a:t") => {
                            in_text = false;
                            if !current.trim().is_empty() { texts.push(current.trim().to_string()); }
                            current.clear();
                        }
                        Ok(Event::Eof) => break,
                        _ => {}
                    }
                }
                
                if !texts.is_empty() {
                    md.push_str(&format!("## Slide {}\n\n{}\n\n", idx + 1, texts.join(" ")));
                }
            }
        }
        
        Ok(md)
    }
    
    /// PDF 文本提取（使用 pdf-extract 库）
    fn convert_pdf(path: &Path) -> Result<String, String> {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read PDF: {}", e))?;
        
        match pdf_extract::extract_text_from_mem(&bytes) {
            Ok(text) => {
                if text.trim().is_empty() {
                    // pdf-extract 返回空文本时，尝试备用方案
                    Self::convert_pdf_fallback(&bytes)
                } else {
                    Ok(text)
                }
            },
            Err(e) => {
                tracing::warn!("pdf-extract failed ({}), trying fallback", e);
                Self::convert_pdf_fallback(&bytes)
            }
        }
    }
    
    /// PDF 备用解析（当 pdf-extract 失败时）
    fn convert_pdf_fallback(bytes: &[u8]) -> Result<String, String> {
        let mut text = String::new();
        let mut current = Vec::new();
        let mut in_stream = false;
        
        for (i, &byte) in bytes.iter().enumerate() {
            if i >= 6 && &bytes[i-6..i] == b"stream" {
                in_stream = true;
                continue;
            }
            if i >= 9 && &bytes[i-9..i] == b"endstream" {
                in_stream = false;
                if current.len() > 4 {
                    if let Ok(s) = String::from_utf8(current.clone()) {
                        text.push_str(&s);
                        text.push('\n');
                    }
                }
                current.clear();
                continue;
            }
            
            if in_stream {
                if byte >= 0x20 && byte < 0x7F || byte >= 0x80 {
                    current.push(byte);
                } else if current.len() > 4 {
                    if let Ok(s) = String::from_utf8(current.clone()) {
                        text.push_str(&s);
                        text.push(' ');
                    }
                    current.clear();
                }
            }
        }
        
        if text.trim().is_empty() {
            Err("No readable text found in PDF".to_string())
        } else {
            Ok(text)
        }
    }
    
    /// RTF 转纯文本（strip 标签）
    fn convert_rtf(path: &Path) -> Result<String, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read RTF: {}", e))?;
        
        // 简单的 RTF 标签剥离
        let mut result = String::new();
        let mut in_brace = 0;
        let mut chars = content.chars().peekable();
        
        // 跳过 {\rtf 头部
        while let Some(c) = chars.next() {
            if c == '{' {
                in_brace += 1;
            } else if c == '}' {
                in_brace -= 1;
            } else if in_brace > 0 {
                if c == '\\' {
                    // 读取控制字
                    let mut control_word = String::new();
                    while let Some(&next) = chars.peek() {
                        if next.is_alphabetic() {
                            control_word.push(next);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    // 跳过控制字后的空格或数字
                    while let Some(&next) = chars.peek() {
                        if next == ' ' || next.is_numeric() || next == '-' {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    // 换行符
                    if control_word == "par" || control_word == "line" {
                        result.push('\n');
                    }
                } else if c != '\n' && c != '\r' {
                    result.push(c);
                }
            }
        }
        
        Ok(result.trim().to_string())
    }
    
    /// HTML 转纯文本
    fn convert_html(path: &Path) -> Result<String, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read HTML: {}", e))?;
        
        // 使用 scraper 解析 HTML
        use scraper::{Html, Selector};
        
        let document = Html::parse_document(&content);
        
        // 提取 title
        let title = document
            .select(&Selector::parse("title").unwrap())
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default();
        
        // 提取 body 文本
        let body_text = document
            .select(&Selector::parse("body").unwrap())
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_else(|| {
                // 如果没有 body，提取所有文本
                document.root_element().text().collect::<Vec<_>>().join(" ")
            });
        
        let mut md = String::new();
        if !title.trim().is_empty() {
            md.push_str(&format!("# {}\n\n", title.trim()));
        }
        md.push_str(body_text.trim());
        
        Ok(md)
    }
    
    /// EPUB 转纯文本（EPUB 是 zip 包含 XHTML）
    fn convert_epub(path: &Path) -> Result<String, String> {
        use zip::ZipArchive;
        
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
        
        let mut md = String::new();
        
        // 找到所有 XHTML 文件
        let mut xhtml_files: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
            .filter(|n| n.ends_with(".xhtml") || n.ends_with(".html") || n.ends_with(".htm"))
            .collect();
        xhtml_files.sort();
        
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
        
        for xhtml_name in &xhtml_files {
            if let Ok(mut file) = archive.by_name(xhtml_name) {
                let mut xml = String::new();
                std::io::Read::read_to_string(&mut file, &mut xml).ok();
                
                // 简单提取文本（类似 HTML 处理）
                let text = Self::strip_html_tags(&xml);
                if !text.trim().is_empty() {
                    md.push_str(&text);
                    md.push_str("\n\n");
                }
            }
        }
        
        Ok(md)
    }
    
    /// ODT 转纯文本（ODT 是 zip 包含 content.xml）
    fn convert_odt(path: &Path) -> Result<String, String> {
        use zip::ZipArchive;
        use quick_xml::Reader;
        use quick_xml::events::Event;
        use quick_xml::name::QName;
        
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
        
        let mut content_xml = String::new();
        if let Ok(mut file) = archive.by_name("content.xml") {
            std::io::Read::read_to_string(&mut file, &mut content_xml).ok();
        }
        
        let mut reader = Reader::from_str(&content_xml);
        let mut texts = Vec::new();
        let mut current = String::new();
        let mut in_text = false;
        
        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) if e.name() == QName(b"text:p") || e.name() == QName(b"text:h") => {
                    in_text = true;
                }
                Ok(Event::Text(ref e)) if in_text => {
                    if let Ok(t) = e.unescape() { current.push_str(&t); }
                }
                Ok(Event::End(ref e)) if e.name() == QName(b"text:p") || e.name() == QName(b"text:h") => {
                    in_text = false;
                    if !current.trim().is_empty() {
                        texts.push(current.trim().to_string());
                    }
                    current.clear();
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
        }
        
        Ok(texts.join("\n\n"))
    }
    
    /// ODS 转纯文本（类似 xlsx 处理）
    fn convert_ods(path: &Path) -> Result<String, String> {
        use calamine::{Reader, open_workbook, Ods, Data};
        
        let mut workbook: Ods<_> = open_workbook(path)
            .map_err(|e| format!("Failed to open ODS: {}", e))?;
        
        let mut md = String::new();
        
        for sheet_name in workbook.sheet_names().to_owned() {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                md.push_str(&format!("## {}\n\n", sheet_name));
                
                let mut rows: Vec<Vec<String>> = Vec::new();
                for row in range.rows() {
                    rows.push(row.iter().map(|c| match c {
                        Data::String(s) => s.clone(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        _ => String::new(),
                    }).collect());
                }
                
                if let Some(header) = rows.first() {
                    md.push_str(&format!("| {} |\n", header.join(" | ")));
                    md.push_str(&format!("| {} |\n", header.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")));
                }
                for row in rows.iter().skip(1) {
                    md.push_str(&format!("| {} |\n", row.join(" | ")));
                }
                md.push('\n');
            }
        }
        
        Ok(md)
    }
    
    /// ODP 转纯文本（类似 pptx 处理）
    fn convert_odp(path: &Path) -> Result<String, String> {
        use zip::ZipArchive;
        use quick_xml::Reader;
        use quick_xml::events::Event;
        use quick_xml::name::QName;
        
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
        
        // ODP 的幻灯片在 content.xml 中
        let mut content_xml = String::new();
        if let Ok(mut file) = archive.by_name("content.xml") {
            std::io::Read::read_to_string(&mut file, &mut content_xml).ok();
        }
        
        let mut reader = Reader::from_str(&content_xml);
        let mut slides = Vec::new();
        let mut current_slide = Vec::new();
        let mut current = String::new();
        let mut in_text = false;
        
        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) if e.name() == QName(b"draw:page") => {
                    current_slide = Vec::new();
                }
                Ok(Event::Start(ref e)) if e.name() == QName(b"text:p") => {
                    in_text = true;
                }
                Ok(Event::Text(ref e)) if in_text => {
                    if let Ok(t) = e.unescape() { current.push_str(&t); }
                }
                Ok(Event::End(ref e)) if e.name() == QName(b"text:p") => {
                    in_text = false;
                    if !current.trim().is_empty() {
                        current_slide.push(current.trim().to_string());
                    }
                    current.clear();
                }
                Ok(Event::End(ref e)) if e.name() == QName(b"draw:page") => {
                    if !current_slide.is_empty() {
                        slides.push(current_slide.join(" "));
                    }
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
        }
        
        let mut md = String::new();
        for (idx, slide_text) in slides.iter().enumerate() {
            md.push_str(&format!("## Slide {}\n\n{}\n\n", idx + 1, slide_text));
        }
        
        Ok(md)
    }
    
    /// 旧版 .doc 格式（纯文本提取，不支持格式）
    fn convert_doc_legacy(path: &Path) -> Result<String, String> {
        // .doc 是二进制格式，简单提取可读文本
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read DOC: {}", e))?;
        
        // 提取连续的可读 ASCII/UTF-8 文本
        let mut text = String::new();
        let mut current = Vec::new();
        
        for &byte in &bytes {
            if byte >= 0x20 && byte < 0x7F || byte >= 0x80 {
                current.push(byte);
            } else {
                if current.len() > 4 {
                    if let Ok(s) = String::from_utf8(current.clone()) {
                        text.push_str(&s);
                        text.push('\n');
                    }
                }
                current.clear();
            }
        }
        
        if current.len() > 4 {
            if let Ok(s) = String::from_utf8(current) {
                text.push_str(&s);
            }
        }
        
        Ok(text)
    }
    
    /// 旧版 .ppt 格式（简单文本提取）
    fn convert_ppt_legacy(path: &Path) -> Result<String, String> {
        // .ppt 是二进制格式，简单提取可读文本
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read PPT: {}", e))?;
        
        let mut text = String::new();
        let mut current = Vec::new();
        
        for &byte in &bytes {
            if byte >= 0x20 && byte < 0x7F || byte >= 0x80 {
                current.push(byte);
            } else {
                if current.len() > 4 {
                    if let Ok(s) = String::from_utf8(current.clone()) {
                        text.push_str(&s);
                        text.push(' ');
                    }
                }
                current.clear();
            }
        }
        
        Ok(text)
    }
    
    /// 剥离 HTML 标签的辅助函数
    fn strip_html_tags(html: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        let mut in_script = false;
        let mut in_style = false;
        
        for c in html.chars() {
            if c == '<' {
                in_tag = true;
                // 检查是否是 script 或 style 标签
                let lower: String = html[result.len()..].to_lowercase();
                if lower.starts_with("<script") {
                    in_script = true;
                } else if lower.starts_with("<style") {
                    in_style = true;
                }
            } else if c == '>' {
                in_tag = false;
                if in_script {
                    let lower: String = html[result.len()..].to_lowercase();
                    if lower.contains("</script") {
                        in_script = false;
                    }
                }
                if in_style {
                    let lower: String = html[result.len()..].to_lowercase();
                    if lower.contains("</style") {
                        in_style = false;
                    }
                }
            } else if !in_tag && !in_script && !in_style {
                result.push(c);
            }
        }
        
        result
    }
}
