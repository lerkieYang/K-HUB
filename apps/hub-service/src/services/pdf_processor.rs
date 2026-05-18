/// PDF高级处理 — poppler提取 + Tesseract OCR
/// 提供比 pdf-extract 更好的文本提取质量

use std::path::Path;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// PDF处理配置
#[derive(Debug, Clone)]
pub struct PdfConfig {
    /// pdftotext 可执行文件路径（None则自动查找）
    pub pdftotext_path: Option<String>,
    /// tesseract 可执行文件路径（None则自动查找）
    pub tesseract_path: Option<String>,
    /// tesseract 语言包（默认中文+英文）
    pub ocr_lang: String,
    /// 触发OCR的最小文字数阈值（提取文字少于此数时触发OCR）
    pub ocr_threshold: usize,
    /// 是否启用OCR
    pub ocr_enabled: bool,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            pdftotext_path: None,
            tesseract_path: None,
            ocr_lang: "chi_sim+eng".to_string(),
            ocr_threshold: 50,
            ocr_enabled: true,
        }
    }
}

/// PDF处理器
pub struct PdfProcessor {
    config: PdfConfig,
}

impl PdfProcessor {
    pub fn new(config: PdfConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(PdfConfig::default())
    }

    /// 转换PDF为文本（完整管道：poppler提取 → 检查质量 → 可选OCR）
    pub fn convert(&self, path: &Path) -> Result<String, String> {
        let bytes = std::fs::read(path)
            .map_err(|e| format!("Failed to read PDF: {}", e))?;

        // 步骤1: 尝试 poppler (pdftotext) 提取
        let text = self.extract_with_poppler(path).unwrap_or_default();

        // 步骤2: 检查提取质量
        let clean_text = self.clean_pdf_text(&text);
        let char_count = clean_text.chars().filter(|c| !c.is_whitespace()).count();

        // 检查是否为可读文本（不是二进制垃圾）
        if char_count >= self.config.ocr_threshold && self.is_readable_text(&clean_text) {
            // 文字足够且可读，直接返回
            return Ok(clean_text);
        }

        // 步骤3: 文字不足或不可读，尝试 pdf-extract 作为备选
        let fallback_text = self.extract_with_pdf_extract(&bytes);
        let fallback_clean = self.clean_pdf_text(&fallback_text);
        let fallback_count = fallback_clean.chars().filter(|c| !c.is_whitespace()).count();

        if fallback_count >= self.config.ocr_threshold && self.is_readable_text(&fallback_clean) {
            return Ok(fallback_clean);
        }

        // 步骤4: 两个提取器都不够或不可读，尝试OCR
        if self.config.ocr_enabled {
            match self.extract_with_ocr(path) {
                Ok(ocr_text) if !ocr_text.trim().is_empty() => {
                    let ocr_clean = self.clean_pdf_text(&ocr_text);
                    // 合并：poppler提取的文字 + OCR文字（去重）
                    let merged = self.merge_texts(&clean_text, &ocr_clean);
                    return Ok(merged);
                }
                _ => {}
            }
        }

        // 步骤5: 所有方法都失败，返回最好的结果
        if fallback_count > char_count && self.is_readable_text(&fallback_clean) {
            Ok(fallback_clean)
        } else if !clean_text.trim().is_empty() && self.is_readable_text(&clean_text) {
            Ok(clean_text)
        } else {
            Err("No readable text found in PDF (tried poppler, pdf-extract, OCR)".to_string())
        }
    }

    /// 检查文本是否可读（不是二进制垃圾）
    fn is_readable_text(&self, text: &str) -> bool {
        if text.is_empty() {
            return false;
        }

        // 检查是否包含明显的二进制/图片标记
        let binary_markers = [
            "cHRM", "IDAT", "IHDR", "PLTE", "gAMA", "sRGB",
            "JFIF", "Exif", "ICC_PROFILE", "GIF89", "GIF87",
            "RIFF", "WEBP", "tEXtSoftware",
            "[Content_Types]", "_rels/", "theme/",
            "MSWordDoc", "Word.Document", "KSOProductBuildVer",
        ];

        for marker in &binary_markers {
            if text.contains(marker) {
                return false;
            }
        }

        let chars: Vec<char> = text.chars().collect();
        let total = chars.len();

        // 统计CJK字符（中文文档的核心内容）
        let cjk_count = chars.iter().filter(|&&c| {
            (c >= '\u{4E00}' && c <= '\u{9FFF}') || // CJK统一汉字
            (c >= '\u{3400}' && c <= '\u{4DBF}')    // CJK扩展A
        }).count();

        // 如果有CJK字符，认为是可读的（中文文档）
        if cjk_count > 10 {
            return true;
        }

        // 统计连续字母序列（单词）
        let mut word_count = 0;
        let mut in_word = false;
        for &c in &chars {
            if c.is_ascii_alphabetic() {
                if !in_word {
                    word_count += 1;
                    in_word = true;
                }
            } else {
                in_word = false;
            }
        }

        // 如果有超过5个单词，认为是可读的（英文文档）
        if word_count > 5 {
            return true;
        }

        // 否则认为是二进制垃圾
        false
    }

    /// 使用 poppler (pdftotext) 提取文本
    fn extract_with_poppler(&self, path: &Path) -> Result<String, String> {
        let pdftotext = self.find_pdftotext()?;

        let output = Command::new(&pdftotext)
            .arg("-layout")  // 保留布局
            .arg("-enc")
            .arg("UTF-8")
            .arg(path)
            .arg("-")  // 输出到stdout
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("pdftotext failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("pdftotext error: {}", stderr));
        }

        String::from_utf8(output.stdout)
            .map_err(|e| format!("pdftotext output not UTF-8: {}", e))
    }

    /// 使用 pdf-extract 库提取文本（备选）
    fn extract_with_pdf_extract(&self, bytes: &[u8]) -> String {
        pdf_extract::extract_text_from_mem(bytes).unwrap_or_default()
    }

    /// 使用 Tesseract OCR 提取文本
    fn extract_with_ocr(&self, path: &Path) -> Result<String, String> {
        let tesseract = self.find_tesseract()?;

        // Tesseract 需要图片输入，先用 pdftoppm 转图片
        let temp_dir = std::env::temp_dir().join("khub_ocr");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;

        // 用 pdftoppm 把 PDF 转为 PNG
        let pdftoppm = self.find_pdftoppm()?;
        let output_prefix = temp_dir.join("page");

        let status = Command::new(&pdftoppm)
            .arg("-png")
            .arg("-r")
            .arg("300")  // 300 DPI
            .arg(path)
            .arg(&output_prefix)
            .creation_flags(CREATE_NO_WINDOW)
            .status()
            .map_err(|e| format!("pdftoppm failed: {}", e))?;

        if !status.success() {
            return Err("pdftoppm failed".to_string());
        }

        // 找到生成的图片文件
        let mut image_files: Vec<_> = std::fs::read_dir(&temp_dir)
            .map_err(|e| format!("Failed to read temp dir: {}", e))?
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.starts_with("page") && name.ends_with(".png")
            })
            .collect();
        image_files.sort_by_key(|e| e.path());

        let mut all_text = String::new();

        // 设置 TESSDATA_PREFIX 环境变量
        let tessdata_prefix = self.find_tessdata_prefix();

        for image_file in &image_files {
            let image_path = image_file.path();

            // 用 tesseract 识别
            let mut cmd = Command::new(&tesseract);
            cmd.arg(&image_path)
                .arg("stdout")
                .arg("-l")
                .arg(&self.config.ocr_lang)
                .arg("--psm")
                .arg("6")  // 假设为统一的文本块
                .creation_flags(CREATE_NO_WINDOW);

            // 设置 TESSDATA_PREFIX
            if let Some(ref prefix) = tessdata_prefix {
                cmd.env("TESSDATA_PREFIX", prefix);
            }

            let output = cmd.output()
                .map_err(|e| format!("tesseract failed: {}", e))?;

            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                if !text.trim().is_empty() {
                    all_text.push_str(&text);
                    all_text.push('\n');
                }
            }

            // 清理临时图片
            let _ = std::fs::remove_file(&image_path);
        }

        // 清理临时目录
        let _ = std::fs::remove_dir(&temp_dir);

        if all_text.trim().is_empty() {
            Err("OCR produced no text".to_string())
        } else {
            Ok(all_text)
        }
    }

    /// 清理PDF提取的文本
    fn clean_pdf_text(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len());

        for line in text.lines() {
            let trimmed = line.trim();
            // 跳过空行
            if trimmed.is_empty() {
                if !result.ends_with("\n\n") {
                    result.push('\n');
                }
                continue;
            }

            result.push_str(trimmed);
            result.push('\n');
        }

        // 压缩连续空行
        let cleaner = crate::services::content_cleaner::ContentCleaner::with_defaults();
        let cleaned = cleaner.compress_blank_lines(&result);
        let cleaned = cleaner.trim_trailing_spaces(&cleaned);

        cleaned
    }

    /// 合并两段文本（去重）
    fn merge_texts(&self, text1: &str, text2: &str) -> String {
        if text1.trim().is_empty() {
            return text2.to_string();
        }
        if text2.trim().is_empty() {
            return text1.to_string();
        }

        // 简单合并：如果text2包含text1的大部分内容，只用text2
        let lines1: Vec<&str> = text1.lines().collect();
        let lines2: Vec<&str> = text2.lines().collect();

        let overlap = lines1.iter()
            .filter(|l| !l.trim().is_empty())
            .filter(|l| lines2.iter().any(|l2| l2.contains(l.trim())))
            .count();

        let total = lines1.iter().filter(|l| !l.trim().is_empty()).count();

        if total > 0 && overlap as f64 / total as f64 > 0.5 {
            // 超过50%重叠，用更长的那个
            if text2.len() > text1.len() {
                text2.to_string()
            } else {
                text1.to_string()
            }
        } else {
            // 合并两段
            format!("{}\n\n{}", text1, text2)
        }
    }

    /// 获取应用资源目录
    fn get_app_resource_dir(&self) -> Option<std::path::PathBuf> {
        std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .map(|p| p.join("resources"))
    }

    /// 查找 pdftotext 可执行文件
    fn find_pdftotext(&self) -> Result<String, String> {
        if let Some(ref path) = self.config.pdftotext_path {
            return Ok(path.clone());
        }

        // 尝试常见路径（优先应用目录内的poppler）
        let app_resources = self.get_app_resource_dir();
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();

        let mut candidates = Vec::new();

        // 优先：应用目录内的poppler
        if let Some(ref resources) = app_resources {
            let bundled = resources.join("poppler").join("pdftotext.exe");
            candidates.push(bundled.to_string_lossy().to_string());
        }

        // 其次：系统PATH、常见安装路径
        candidates.push("pdftotext.exe".to_string());
        candidates.push(r"C:\Program Files\poppler\bin\pdftotext.exe".to_string());
        candidates.push(format!(r"{}\Microsoft\WinGet\Packages\oschwartz10612.Poppler_Microsoft.Winget.Source_8wekyb3d8bbwe\poppler-25.07.0\Library\bin\pdftotext.exe", local_app_data));

        for candidate in &candidates {
            if !candidate.is_empty() && Command::new(candidate).arg("-v").creation_flags(CREATE_NO_WINDOW).output().is_ok() {
                return Ok(candidate.clone());
            }
        }

        Err("pdftotext not found. Install poppler (winget install oschwartz10612.Poppler)".to_string())
    }

    /// 查找 pdftoppm 可执行文件
    fn find_pdftoppm(&self) -> Result<String, String> {
        let app_resources = self.get_app_resource_dir();
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();

        let mut candidates = Vec::new();

        // 优先：应用目录内的poppler
        if let Some(ref resources) = app_resources {
            let bundled = resources.join("poppler").join("pdftoppm.exe");
            candidates.push(bundled.to_string_lossy().to_string());
        }

        candidates.push("pdftoppm.exe".to_string());
        candidates.push(r"C:\Program Files\poppler\bin\pdftoppm.exe".to_string());
        candidates.push(format!(r"{}\Microsoft\WinGet\Packages\oschwartz10612.Poppler_Microsoft.Winget.Source_8wekyb3d8bbwe\poppler-25.07.0\Library\bin\pdftoppm.exe", local_app_data));

        for candidate in &candidates {
            if !candidate.is_empty() && Command::new(candidate).arg("-v").creation_flags(CREATE_NO_WINDOW).output().is_ok() {
                return Ok(candidate.clone());
            }
        }

        Err("pdftoppm not found. Install poppler (winget install oschwartz10612.Poppler)".to_string())
    }

    /// 查找 tesseract 可执行文件
    fn find_tesseract(&self) -> Result<String, String> {
        if let Some(ref path) = self.config.tesseract_path {
            return Ok(path.clone());
        }

        let app_resources = self.get_app_resource_dir();

        let mut candidates = Vec::new();

        // 优先：应用目录内的tesseract
        if let Some(ref resources) = app_resources {
            let bundled = resources.join("tesseract").join("tesseract.exe");
            candidates.push(bundled.to_string_lossy().to_string());
        }

        candidates.push("tesseract.exe".to_string());
        candidates.push(r"C:\Program Files\Tesseract-OCR\tesseract.exe".to_string());

        for candidate in &candidates {
            if !candidate.is_empty() && Command::new(candidate).arg("--version").creation_flags(CREATE_NO_WINDOW).output().is_ok() {
                return Ok(candidate.clone());
            }
        }

        Err("tesseract not found. Install Tesseract (winget install UB-Mannheim.TesseractOCR)".to_string())
    }

    /// 查找 tessdata 目录
    fn find_tessdata_prefix(&self) -> Option<String> {
        let app_resources = self.get_app_resource_dir();
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();

        let mut candidates = Vec::new();

        // 优先：应用目录内的tesseract
        if let Some(ref resources) = app_resources {
            let bundled = resources.join("tesseract");
            candidates.push(bundled.to_string_lossy().to_string());
        }

        candidates.push(format!(r"{}\Tesseract-OCR", local_app_data));
        candidates.push(r"C:\Program Files\Tesseract-OCR".to_string());

        for candidate in &candidates {
            let tessdata_dir = std::path::Path::new(candidate).join("tessdata");
            if tessdata_dir.exists() {
                return Some(candidate.clone());
            }
        }

        None
    }

    /// 检查 poppler 是否可用
    pub fn has_poppler(&self) -> bool {
        self.find_pdftotext().is_ok()
    }

    /// 检查 tesseract 是否可用
    pub fn has_tesseract(&self) -> bool {
        self.find_tesseract().is_ok()
    }

    /// 获取可用工具状态
    pub fn tool_status(&self) -> serde_json::Value {
        serde_json::json!({
            "poppler": self.has_poppler(),
            "tesseract": self.has_tesseract(),
            "ocr_enabled": self.config.ocr_enabled,
            "ocr_lang": self.config.ocr_lang,
            "ocr_threshold": self.config.ocr_threshold,
        })
    }
}
