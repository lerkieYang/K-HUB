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
    
    /// 转换文档（含自动清洗）
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

        // 转换后自动清洗
        let cleaner = crate::services::content_cleaner::ContentCleaner::with_defaults();
        let cleaned = cleaner.clean_knowledge(&content);

        std::fs::write(&target, cleaned)
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

        // A6: 提取图片
        let image_dir = Self::extract_docx_images(path).unwrap_or_default();

        let mut md = String::new();

        for child in docx.document.children {
            match child {
                docx_rs::DocumentChild::Paragraph(para) => {
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
                        match content {
                            docx_rs::ParagraphChild::Run(run) => {
                                for run_child in &run.children {
                                    if let docx_rs::RunChild::Text(t) = run_child {
                                        text.push_str(&t.text);
                                    }
                                    // A6: 检测图片引用
                                    if let docx_rs::RunChild::Drawing(_drawing) = run_child {
                                        // 图片已提取到image_dir，插入引用
                                        if !image_dir.is_empty() {
                                            let img_idx = Self::count_images_in_md(&md);
                                            let img_path = format!("{}/image_{}.png", image_dir, img_idx + 1);
                                            if std::path::Path::new(&img_path).exists() {
                                                text.push_str(&format!("![图{}]({})", img_idx + 1, img_path));
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
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
                // A5: 表格提取
                docx_rs::DocumentChild::Table(table) => {
                    md.push_str(&Self::convert_docx_table(&table));
                    md.push('\n');
                }
                _ => {}
            }
        }

        Ok(md)
    }

    /// A5: 将docx表格转换为Markdown表格
    fn convert_docx_table(table: &docx_rs::Table) -> String {
        let mut rows = Vec::new();

        for row_child in &table.rows {
            // TableChild::TableRow(TableRow)
            let row = match row_child {
                docx_rs::TableChild::TableRow(row) => row,
                _ => continue,
            };

            let mut cells = Vec::new();
            for cell_child in &row.cells {
                // TableRowChild::TableCell(TableCell)
                let cell = match cell_child {
                    docx_rs::TableRowChild::TableCell(cell) => cell,
                    _ => continue,
                };

                let mut cell_text = String::new();
                for content in &cell.children {
                    if let docx_rs::TableCellContent::Paragraph(para) = content {
                        for para_content in &para.children {
                            if let docx_rs::ParagraphChild::Run(run) = para_content {
                                for run_child in &run.children {
                                    if let docx_rs::RunChild::Text(t) = run_child {
                                        if !cell_text.is_empty() {
                                            cell_text.push(' ');
                                        }
                                        cell_text.push_str(&t.text);
                                    }
                                }
                            }
                        }
                    }
                }
                cells.push(cell_text.trim().to_string());
            }
            rows.push(cells);
        }

        if rows.is_empty() {
            return String::new();
        }

        let mut md = String::new();

        // 第一行作为表头
        if let Some(header) = rows.first() {
            md.push_str(&format!("| {} |\n", header.join(" | ")));
            md.push_str(&format!("| {} |\n", header.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")));
        }

        // 数据行
        for row in rows.iter().skip(1) {
            md.push_str(&format!("| {} |\n", row.join(" | ")));
        }

        md
    }

    /// A6: 从docx文件中提取图片
    fn extract_docx_images(docx_path: &Path) -> Result<String, String> {
        use zip::ZipArchive;

        let file = std::fs::File::open(docx_path)
            .map_err(|e| format!("Failed to open docx: {}", e))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("Failed to read zip: {}", e))?;

        // 图片存放目录: 与docx同级的 .images/ 目录
        let image_dir = docx_path.parent()
            .unwrap_or(std::path::Path::new("."))
            .join(".images")
            .join(docx_path.file_stem().unwrap_or_default().to_string_lossy().to_string());

        let mut image_count = 0;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;
            let name = file.name().to_string();

            // 只提取 word/media/ 下的图片
            if name.starts_with("word/media/") {
                let ext = name.rsplit('.').next().unwrap_or("bin");
                if !matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "bmp" | "wmf" | "emf") {
                    continue;
                }

                image_count += 1;
                let target_dir = &image_dir;
                std::fs::create_dir_all(target_dir)
                    .map_err(|e| format!("Failed to create image dir: {}", e))?;

                let target_path = target_dir.join(format!("image_{}.{}", image_count, ext));
                let mut target_file = std::fs::File::create(&target_path)
                    .map_err(|e| format!("Failed to create image file: {}", e))?;
                std::io::copy(&mut file, &mut target_file)
                    .map_err(|e| format!("Failed to write image: {}", e))?;
            }
        }

        if image_count > 0 {
            Ok(image_dir.to_string_lossy().to_string())
        } else {
            Ok(String::new())
        }
    }

    /// 计算已有的图片引用数量
    fn count_images_in_md(md: &str) -> usize {
        md.matches("![图").count() + md.matches("![](").count()
    }
    
    fn convert_xlsx(path: &Path) -> Result<String, String> {
        use calamine::{Reader, open_workbook, Xlsx, Data};

        let mut workbook: Xlsx<_> = open_workbook(path)
            .map_err(|e| format!("Failed to open: {}", e))?;

        let mut md = String::new();
        let sheet_names = workbook.sheet_names().to_owned();

        // A10: 检测是否所有Sheet表头相同，如果是则合并
        let mut all_headers: Vec<Vec<String>> = Vec::new();
        let mut sheet_data: Vec<(String, Vec<Vec<String>>)> = Vec::new();

        for sheet_name in &sheet_names {
            if let Ok(range) = workbook.worksheet_range(sheet_name) {
                let mut rows: Vec<Vec<String>> = Vec::new();
                for row in range.rows() {
                    rows.push(row.iter().map(|c| match c {
                        Data::String(s) => s.clone(),
                        Data::Float(f) => Self::format_number(f),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        _ => String::new(),
                    }).collect());
                }

                if let Some(header) = rows.first() {
                    all_headers.push(header.clone());
                }
                sheet_data.push((sheet_name.clone(), rows));
            }
        }

        // 检查所有Sheet表头是否相同
        let headers_match = if all_headers.len() > 1 {
            let first = &all_headers[0];
            all_headers.iter().all(|h| h == first)
        } else {
            false
        };

        if headers_match && sheet_data.len() > 1 {
            // 表头相同：合并为一个表格，Sheet名作为额外列
            let header = &all_headers[0];
            md.push_str(&format!("| {} | Sheet |\n", header.join(" | ")));
            md.push_str(&format!("| {} | --- |\n", header.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")));

            for (sheet_name, rows) in &sheet_data {
                for row in rows.iter().skip(1) {
                    // A9: 跳过全空行
                    if row.iter().all(|c| c.trim().is_empty()) {
                        continue;
                    }
                    md.push_str(&format!("| {} | {} |\n", row.join(" | "), sheet_name));
                }
            }
            md.push('\n');
        } else {
            // 表头不同：每个Sheet独立输出
            for (sheet_name, rows) in &sheet_data {
                md.push_str(&format!("## {}\n\n", sheet_name));

                if let Some(header) = rows.first() {
                    md.push_str(&format!("| {} |\n", header.join(" | ")));
                    md.push_str(&format!("| {} |\n", header.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")));
                }
                for row in rows.iter().skip(1) {
                    // A9: 跳过全空行
                    if row.iter().all(|c| c.trim().is_empty()) {
                        continue;
                    }
                    md.push_str(&format!("| {} |\n", row.join(" | ")));
                }
                md.push('\n');
            }
        }

        Ok(md)
    }

    /// 格式化数字，避免科学计数法
    fn format_number(f: &f64) -> String {
        if f.fract() == 0.0 && f.abs() < 1e15 {
            format!("{}", *f as i64)
        } else {
            format!("{}", f)
        }
    }
    
    fn convert_pptx(path: &Path) -> Result<String, String> {
        use zip::ZipArchive;
        use quick_xml::Reader;
        use quick_xml::events::Event;
        use quick_xml::name::QName;

        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

        // 收集slide文件列表
        let mut slide_files: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
            .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml") && !n.contains("slideLayout"))
            .collect();
        slide_files.sort();

        // 收集speaker notes文件列表
        let notes_files: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
            .filter(|n| n.starts_with("ppt/notesSlides/notesSlide") && n.ends_with(".xml"))
            .collect();

        // A6: 提取PPT图片
        let image_dir = Self::extract_pptx_images(path).unwrap_or_default();

        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

        let mut md = String::new();

        for (idx, slide_name) in slide_files.iter().enumerate() {
            // 提取幻灯片文本
            let slide_texts = if let Ok(mut file) = archive.by_name(slide_name) {
                let mut xml = String::new();
                std::io::Read::read_to_string(&mut file, &mut xml).ok();
                Self::extract_pptx_texts(&xml)
            } else {
                Vec::new()
            };

            // 提取speaker notes（如果有）
            let notes_text = {
                let notes_name = format!("ppt/notesSlides/notesSlide{}.xml", idx + 1);
                if let Ok(mut file) = archive.by_name(&notes_name) {
                    let mut xml = String::new();
                    std::io::Read::read_to_string(&mut file, &mut xml).ok();
                    let notes = Self::extract_pptx_texts(&xml);
                    if notes.is_empty() {
                        None
                    } else {
                        Some(notes.join(" "))
                    }
                } else {
                    None
                }
            };

            if slide_texts.is_empty() && notes_text.is_none() {
                continue;
            }

            // 输出Slide结构
            md.push_str(&format!("## Slide {}\n\n", idx + 1));

            // 幻灯片内容：按文本框分组，每组一行
            for text in &slide_texts {
                md.push_str(&format!("{}\n", text));
            }

            // Speaker Notes
            if let Some(notes) = &notes_text {
                md.push_str(&format!("\n> 备注: {}\n", notes));
            }

            md.push('\n');
        }

        Ok(md)
    }

    /// 从PPT XML中提取文本（按文本框分组）
    fn extract_pptx_texts(xml: &str) -> Vec<String> {
        use quick_xml::Reader;
        use quick_xml::events::Event;
        use quick_xml::name::QName;

        let mut reader = Reader::from_str(xml);
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

        texts
    }

    /// A6: 从PPT文件中提取图片
    fn extract_pptx_images(pptx_path: &Path) -> Result<String, String> {
        use zip::ZipArchive;

        let file = std::fs::File::open(pptx_path)
            .map_err(|e| format!("Failed to open pptx: {}", e))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("Failed to read zip: {}", e))?;

        let image_dir = pptx_path.parent()
            .unwrap_or(std::path::Path::new("."))
            .join(".images")
            .join(pptx_path.file_stem().unwrap_or_default().to_string_lossy().to_string());

        let mut image_count = 0;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;
            let name = file.name().to_string();

            if name.starts_with("ppt/media/") {
                let ext = name.rsplit('.').next().unwrap_or("bin");
                if !matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "bmp" | "wmf" | "emf") {
                    continue;
                }

                image_count += 1;
                std::fs::create_dir_all(&image_dir)
                    .map_err(|e| format!("Failed to create image dir: {}", e))?;

                let target_path = image_dir.join(format!("image_{}.{}", image_count, ext));
                let mut target_file = std::fs::File::create(&target_path)
                    .map_err(|e| format!("Failed to create image file: {}", e))?;
                std::io::copy(&mut file, &mut target_file)
                    .map_err(|e| format!("Failed to write image: {}", e))?;
            }
        }

        if image_count > 0 {
            Ok(image_dir.to_string_lossy().to_string())
        } else {
            Ok(String::new())
        }
    }
    
    /// PDF 文本提取（使用 poppler + pdf-extract + OCR 多级管道）
    fn convert_pdf(path: &Path) -> Result<String, String> {
        let processor = crate::services::pdf_processor::PdfProcessor::with_defaults();
        processor.convert(path)
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
        // .doc 是二进制格式，但有些.doc文件实际是.docx（ZIP格式）
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read DOC: {}", e))?;

        // 检测是否为ZIP格式（docx的magic bytes: PK\x03\x04）
        if bytes.len() >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4B && bytes[2] == 0x03 && bytes[3] == 0x04 {
            // 实际是docx格式，用convert_docx处理
            return Self::convert_docx(path);
        }

        // 原始.doc格式，提取可读文本并过滤噪声
        let mut text = String::new();
        let mut current = Vec::new();

        // 更全面的噪声过滤列表
        let noise_patterns = [
            // XML/OOXML
            "<?xml", "<", "PK", "[Content_Types]", "_rels/", "theme/", "drs/",
            "slideLayout", "slideMaster", "xmlns", "w:rPr", "w:pPr", "w:body",
            "w:document", "w:settings", "w:styles", "w:fontTable", "w:numbering",
            "mc:", "w:", "r:", "a:", "v:", "o:", "w10:", "w14:", "w15:",
            // Word内部标记
            "MSWordDoc", "Word.Document", "Normal", "datastoreItem", "schemaRef",
            "KSOProductBuildVer", "Microsoft Office Word", "Microsoft",
            // 图片格式标记
            "cHRM", "tEXtSoftware", "IDAT", "IHDR", "PLTE", "gAMA", "sRGB",
            "JFIF", "Exif", "ICC_PROFILE", "PNG", "GIF89", "GIF87",
            "RIFF", "WEBP", "BM", "GIF",
            // OLE/复合文档
            "Root Entry", "CompObj", "Ole", "ObjInfo",
            // 其他二进制噪声
            "㰢", "㰔", "㰕", "㰖", "㰗", "㰘", // 常见乱码字符
        ];

        for &byte in &bytes {
            // 可打印ASCII或UTF-8多字节
            if byte >= 0x20 && byte < 0x7F || byte >= 0x80 {
                current.push(byte);
            } else {
                if current.len() > 4 {
                    if let Ok(s) = String::from_utf8(current.clone()) {
                        let trimmed = s.trim();
                        // 检查是否为噪声
                        let is_noise = trimmed.is_empty()
                            || trimmed.len() <= 2
                            || noise_patterns.iter().any(|p| trimmed.contains(p))
                            || is_binary_garbage(trimmed);

                        if !is_noise {
                            text.push_str(trimmed);
                            text.push('\n');
                        }
                    }
                }
                current.clear();
            }
        }

        // 处理最后一段
        if current.len() > 4 {
            if let Ok(s) = String::from_utf8(current) {
                let trimmed = s.trim();
                let is_noise = trimmed.is_empty()
                    || trimmed.len() <= 2
                    || noise_patterns.iter().any(|p| trimmed.contains(p))
                    || is_binary_garbage(trimmed);

                if !is_noise {
                    text.push_str(trimmed);
                }
            }
        }

        // 压缩连续空行
        let cleaner = crate::services::content_cleaner::ContentCleaner::with_defaults();
        let cleaned = cleaner.compress_blank_lines(&text);
        let cleaned = cleaner.trim_trailing_spaces(&cleaned);

        if cleaned.trim().is_empty() {
            return Err("No readable text found in DOC file".to_string());
        }

        Ok(cleaned)
    }
    
    /// 旧版 .ppt 格式（文本提取，过滤噪声）
    fn convert_ppt_legacy(path: &Path) -> Result<String, String> {
        // .ppt 是二进制格式，但有些.ppt文件实际是.pptx（ZIP格式）
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read PPT: {}", e))?;

        // 检测是否为ZIP格式（pptx的magic bytes: PK\x03\x04）
        if bytes.len() >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4B && bytes[2] == 0x03 && bytes[3] == 0x04 {
            // 实际是pptx格式，用convert_pptx处理
            return Self::convert_pptx(path);
        }

        // 原始.ppt格式，提取可读文本并过滤噪声
        let mut text = String::new();
        let mut current = Vec::new();

        // 更全面的噪声过滤列表
        let noise_patterns = [
            // XML/OOXML
            "<?xml", "<", "PK", "[Content_Types]", "_rels/", "theme/", "drs/",
            "slideLayout", "slideMaster", "xmlns", "a:", "r:", "p:",
            // PowerPoint内部标记
            "PowerPoint", "PP97_DUALSTORAGE", "Current User", "Persistent Directory",
            "DocumentSummaryInformation", "SummaryInformation",
            // 图片格式标记
            "cHRM", "tEXtSoftware", "IDAT", "IHDR", "PLTE", "gAMA", "sRGB",
            "JFIF", "Exif", "ICC_PROFILE", "PNG", "GIF89", "GIF87",
            "RIFF", "WEBP", "BM", "GIF",
            // OLE/复合文档
            "Root Entry", "CompObj", "Ole", "ObjInfo",
        ];

        for &byte in &bytes {
            if byte >= 0x20 && byte < 0x7F || byte >= 0x80 {
                current.push(byte);
            } else {
                if current.len() > 4 {
                    if let Ok(s) = String::from_utf8(current.clone()) {
                        let trimmed = s.trim();
                        // 检查是否为噪声
                        let is_noise = trimmed.is_empty()
                            || trimmed.len() <= 2
                            || noise_patterns.iter().any(|p| trimmed.contains(p))
                            || is_binary_garbage(trimmed);

                        if !is_noise {
                            text.push_str(trimmed);
                            text.push(' ');
                        }
                    }
                }
                current.clear();
            }
        }

        // 处理最后一段
        if current.len() > 4 {
            if let Ok(s) = String::from_utf8(current) {
                let trimmed = s.trim();
                let is_noise = trimmed.is_empty()
                    || trimmed.len() <= 2
                    || noise_patterns.iter().any(|p| trimmed.contains(p))
                    || is_binary_garbage(trimmed);

                if !is_noise {
                    text.push_str(trimmed);
                }
            }
        }

        // 压缩连续空格
        let cleaner = crate::services::content_cleaner::ContentCleaner::with_defaults();
        let cleaned = cleaner.compress_blank_lines(&text);
        let cleaned = cleaner.trim_trailing_spaces(&cleaned);

        if cleaned.trim().is_empty() {
            return Err("No readable text found in PPT file".to_string());
        }

        Ok(cleaned)
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

/// 检测文本是否为二进制垃圾（乱码）
/// 通过检查字符分布和连续特殊字符来判断
fn is_binary_garbage(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }

    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();

    // 统计各类字符
    let mut ascii_printable = 0;
    let mut cjk = 0;
    let mut control_or_special = 0;
    let mut consecutive_special = 0;
    let mut max_consecutive_special = 0;

    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_graphic() || c == ' ' {
            ascii_printable += 1;
            consecutive_special = 0;
        } else if c >= '\u{4E00}' && c <= '\u{9FFF}' {
            // CJK字符
            cjk += 1;
            consecutive_special = 0;
        } else if c >= '\u{3000}' && c <= '\u{303F}' {
            // CJK标点
            cjk += 1;
            consecutive_special = 0;
        } else {
            control_or_special += 1;
            consecutive_special += 1;
            if consecutive_special > max_consecutive_special {
                max_consecutive_special = consecutive_special;
            }
        }
    }

    // 如果超过30%是控制字符/特殊字符，判定为垃圾
    if total > 0 && (control_or_special as f64 / total as f64) > 0.3 {
        return true;
    }

    // 如果有超过5个连续特殊字符，判定为垃圾
    if max_consecutive_special > 5 {
        return true;
    }

    // 如果没有CJK字符且没有可读ASCII，判定为垃圾
    if cjk == 0 && ascii_printable < 5 {
        return true;
    }

    // 检查是否包含大量罕见Unicode字符（常见于二进制数据）
    let rare_chars = chars.iter().filter(|&&c| {
        (c >= '\u{E000}' && c <= '\u{F8FF}') || // 私用区
        (c >= '\u{FFF0}' && c <= '\u{FFFF}') || // 特殊区
        (c >= '\u{2400}' && c <= '\u{243F}') || // 控制图片
        (c >= '\u{2500}' && c <= '\u{257F}') && total < 20  // 制表符（短文本中）
    }).count();

    if total > 0 && (rare_chars as f64 / total as f64) > 0.2 {
        return true;
    }

    false
}
