use crate::error::{anyhow, Result};
use lopdf::{Document, Object, ObjectId};
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;

pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct TocEntry {
    pub title: String,
    pub level: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PdfMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_count: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub table_of_contents: Vec<TocEntry>,
}

impl PdfMetadata {
    pub fn from_pdf(path: &Path) -> Result<Self> {
        let doc = Document::load(path)?;

        // 1. 建立页面映射: page_object_id -> page_number (1-based)
        let mut page_map: HashMap<ObjectId, u32> = HashMap::new();
        let pages = doc.get_pages();
        let page_count = pages.len() as u32;
        for (page_num, page_obj_id) in pages {
            page_map.insert(page_obj_id, page_num);
        }

        // 2. 建立命名目标映射: name -> dest_object_id
        let mut dest_map = HashMap::new();
        build_dest_map(&doc, &mut dest_map);

        let mut metadata = PdfMetadata {
            title: None,
            author: None,
            subject: None,
            keywords: None,
            creator: None,
            producer: None,
            created: None,
            modified: None,
            page_count: Some(page_count),
            table_of_contents: Vec::new(),
        };

        if let Ok(info_ref) = doc.trailer.get(b"Info") {
            if let Ok(info_id) = info_ref.as_reference() {
                if let Ok(info_dict) = doc.get_object(info_id).and_then(|obj| obj.as_dict()) {
                    if let Ok(obj) = info_dict.get(b"Title") {
                        if let Ok(bytes) = obj.as_str() {
                            metadata.title = Some(String::from_utf8_lossy(bytes).into_owned());
                        }
                    }
                    if let Ok(obj) = info_dict.get(b"Author") {
                        if let Ok(bytes) = obj.as_str() {
                            metadata.author = Some(String::from_utf8_lossy(bytes).into_owned());
                        }
                    }
                    if let Ok(obj) = info_dict.get(b"Subject") {
                        if let Ok(bytes) = obj.as_str() {
                            metadata.subject = Some(String::from_utf8_lossy(bytes).into_owned());
                        }
                    }
                    if let Ok(obj) = info_dict.get(b"Keywords") {
                        if let Ok(bytes) = obj.as_str() {
                            metadata.keywords = Some(String::from_utf8_lossy(bytes).into_owned());
                        }
                    }
                    if let Ok(obj) = info_dict.get(b"Creator") {
                        if let Ok(bytes) = obj.as_str() {
                            metadata.creator = Some(String::from_utf8_lossy(bytes).into_owned());
                        }
                    }
                    if let Ok(obj) = info_dict.get(b"Producer") {
                        if let Ok(bytes) = obj.as_str() {
                            metadata.producer = Some(String::from_utf8_lossy(bytes).into_owned());
                        }
                    }
                    if let Ok(obj) = info_dict.get(b"CreationDate") {
                        if let Ok(bytes) = obj.as_str() {
                            let s = String::from_utf8_lossy(bytes).into_owned();
                            metadata.created = parse_pdf_date(&s);
                        }
                    }
                    if let Ok(obj) = info_dict.get(b"ModDate") {
                        if let Ok(bytes) = obj.as_str() {
                            let s = String::from_utf8_lossy(bytes).into_owned();
                            metadata.modified = parse_pdf_date(&s);
                        }
                    }
                }
            }
        }

        metadata.table_of_contents = extract_outlines(&doc, &page_map, &dest_map);

        Ok(metadata)
    }

    pub fn to_yaml_frontmatter(&self) -> String {
        let mut yaml = String::from("---\n");

        if let Ok(yaml_str) = serde_yaml::to_string(self) {
            yaml.push_str(&yaml_str);
        }

        yaml.push_str("---\n\n");
        yaml
    }
}

fn extract_outlines(
    doc: &Document,
    page_map: &HashMap<ObjectId, u32>,
    dest_map: &HashMap<String, ObjectId>,
) -> Vec<TocEntry> {
    let mut toc = Vec::new();

    if let Ok(catalog) = doc.catalog() {
        if let Ok(outlines_ref) = catalog.get(b"Outlines") {
            if let Ok(outlines_id) = outlines_ref.as_reference() {
                if let Ok(outlines_dict) = doc.get_object(outlines_id).and_then(|obj| obj.as_dict())
                {
                    if let Ok(first_ref) = outlines_dict.get(b"First") {
                        collect_outline_items(doc, first_ref, 0, &mut toc, page_map, dest_map);
                    }
                }
            }
        }
    }

    toc
}

fn collect_outline_items(
    doc: &Document,
    item_ref: &Object,
    level: u32,
    toc: &mut Vec<TocEntry>,
    page_map: &HashMap<ObjectId, u32>,
    dest_map: &HashMap<String, ObjectId>,
) {
    if let Ok(item_id) = item_ref.as_reference() {
        if let Ok(item_dict) = doc.get_object(item_id).and_then(|obj| obj.as_dict()) {
            let mut title = String::new();
            let mut page = None;

            if let Ok(title_ref_obj) = item_dict.get(b"Title") {
                let title_bytes = if let Ok(title_id) = title_ref_obj.as_reference() {
                    if let Ok(obj) = doc.get_object(title_id) {
                        obj.as_str().ok()
                    } else {
                        None
                    }
                } else {
                    title_ref_obj.as_str().ok()
                };

                if let Some(bytes) = title_bytes {
                    title = decode_pdf_string(bytes);
                }
            }

            // 提取页码
            if let Ok(action_obj) = item_dict.get(b"A").or_else(|_| item_dict.get(b"Dest")) {
                page = extract_page_number(doc, action_obj, page_map, dest_map);
            }

            if !title.is_empty() {
                toc.push(TocEntry { title, level, page });
            }

            if let Ok(first_ref) = item_dict.get(b"First") {
                collect_outline_items(doc, first_ref, level + 1, toc, page_map, dest_map);
            }

            if let Ok(next_ref) = item_dict.get(b"Next") {
                collect_outline_items(doc, next_ref, level, toc, page_map, dest_map);
            }
        }
    }
}

fn build_dest_map(doc: &Document, dest_map: &mut HashMap<String, ObjectId>) {
    if let Ok(catalog) = doc.catalog() {
        if let Ok(names) = catalog.get(b"Names") {
            if let Ok(names_id) = names.as_reference() {
                if let Ok(names_dict) = doc.get_object(names_id).and_then(|obj| obj.as_dict()) {
                    if let Ok(dests) = names_dict.get(b"Dests") {
                        if let Ok(dests_id) = dests.as_reference() {
                            collect_dests_from_tree(doc, dests_id, dest_map);
                        }
                    }
                }
            }
        }
    }
}

fn collect_dests_from_tree(
    doc: &Document,
    node_id: ObjectId,
    dest_map: &mut HashMap<String, ObjectId>,
) {
    if let Ok(node) = doc.get_object(node_id).and_then(|obj| obj.as_dict()) {
        if let Ok(names_array) = node.get(b"Names").and_then(|a| a.as_array()) {
            for i in (0..names_array.len()).step_by(2) {
                if i + 1 < names_array.len() {
                    let name_obj = &names_array[i];
                    let dest_obj = &names_array[i + 1];

                    if let Ok(name_bytes) = name_obj.as_str() {
                        let name = String::from_utf8_lossy(name_bytes).into_owned();
                        if let Ok(dest_id) = dest_obj.as_reference() {
                            dest_map.insert(name, dest_id);
                        }
                    }
                }
            }
        }

        if let Ok(kids_array) = node.get(b"Kids").and_then(|a| a.as_array()) {
            for kid in kids_array {
                if let Ok(kid_id) = kid.as_reference() {
                    collect_dests_from_tree(doc, kid_id, dest_map);
                }
            }
        }
    }
}

fn extract_page_number(
    doc: &Document,
    action_obj: &Object,
    page_map: &HashMap<ObjectId, u32>,
    dest_map: &HashMap<String, ObjectId>,
) -> Option<u32> {
    // 先看看 action_obj 是不是引用，去取目标对象
    let target_dict = if let Ok(action_id) = action_obj.as_reference() {
        doc.get_object(action_id).and_then(|o| o.as_dict()).ok()
    } else {
        action_obj.as_dict().ok()
    };

    let d_value = target_dict.and_then(|dict| dict.get(b"D").ok())?;

    // 看看 D 是字符串（命名目标）还是数组
    if let Ok(d_name_bytes) = d_value.as_str() {
        let d_name = String::from_utf8_lossy(d_name_bytes).into_owned();
        let dest_id = dest_map.get(&d_name)?;
        let dest_dict = doc.get_object(*dest_id).and_then(|o| o.as_dict()).ok()?;
        let d_array = dest_dict.get(b"D").and_then(|a| a.as_array()).ok()?;
        return extract_page_from_array(d_array, page_map);
    } else if let Ok(d_array) = d_value.as_array() {
        return extract_page_from_array(d_array, page_map);
    }

    None
}

fn extract_page_from_array(array: &[Object], page_map: &HashMap<ObjectId, u32>) -> Option<u32> {
    if !array.is_empty() {
        if let Ok(page_obj_id) = array[0].as_reference() {
            return page_map.get(&page_obj_id).copied();
        }
    }
    None
}

fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFE, 0xFF]) && bytes.len() >= 4 {
        let utf16_bytes: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        return String::from_utf16_lossy(&utf16_bytes);
    } else if bytes.starts_with(&[0xFF, 0xFE]) && bytes.len() >= 4 {
        let utf16_bytes: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        return String::from_utf16_lossy(&utf16_bytes);
    }

    String::from_utf8_lossy(bytes).into_owned()
}

fn parse_pdf_date(pdf_date: &str) -> Option<String> {
    let s = pdf_date.trim_start_matches("D:");
    let s = s.split('\'').next().unwrap_or(s);

    if s.len() >= 14 {
        let year = s.get(0..4)?;
        let month = s.get(4..6)?;
        let day = s.get(6..8)?;
        let hour = s.get(8..10)?;
        let minute = s.get(10..12)?;
        let second = s.get(12..14)?;
        Some(format!("{}-{}-{}T{}:{}:{}", year, month, day, hour, minute, second))
    } else if s.len() >= 8 {
        let year = s.get(0..4)?;
        let month = s.get(4..6)?;
        let day = s.get(6..8)?;
        Some(format!("{}-{}-{}", year, month, day))
    } else {
        None
    }
}

/// Check if a string is a URL
pub fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Convert arxiv abs link to pdf link if needed
pub fn normalize_arxiv_url(url: &str) -> String {
    // Check for arxiv.org/abs/xxx and convert to arxiv.org/pdf/xxx
    let re = regex::Regex::new(r"^https?://arxiv\.org/abs/([\w\.\/]+)$").unwrap();
    if let Some(captures) = re.captures(url) {
        let path = captures.get(1).unwrap().as_str();
        return format!("https://arxiv.org/pdf/{}.pdf", path);
    }
    url.to_string()
}

/// Download a PDF from URL and save to a temporary file
pub async fn download_pdf(url: &str) -> Result<NamedTempFile> {
    let client = Client::new();
    let response = client.get(url).send().await?;
    
    if !response.status().is_success() {
        return Err(anyhow!("Failed to download PDF: HTTP status {}", response.status()));
    }
    
    let bytes = response.bytes().await?;
    let mut temp_file = NamedTempFile::new()?;
    std::io::Write::write_all(&mut temp_file, &bytes)?;
    
    Ok(temp_file)
}

/// Split a PDF, keeping only the specified pages (1-based)
pub fn split_pdf(input_path: &Path, output_path: &Path, ranges: &[(u32, u32)]) -> Result<()> {
    let mut doc = Document::load(input_path)?;

    // 1. 收集要保留的页码（1-based）
    let mut page_nums_to_keep = Vec::new();
    for &(start, end) in ranges {
        for page_num in start..=end {
            page_nums_to_keep.push(page_num);
        }
    }

    if page_nums_to_keep.is_empty() {
        return Err(anyhow!("No pages selected for extraction"));
    }

    page_nums_to_keep.sort();
    page_nums_to_keep.dedup();

    // 2. 获取所有页面和 Catalog
    let page_map = doc.get_pages();
    let catalog = doc.catalog()?;
    let pages_id = catalog.get(b"Pages")?.as_reference()?;

    // 3. 找出需要保留的页面对象 ID
    let mut page_ids_to_keep = Vec::new();
    for page_num in &page_nums_to_keep {
        if let Some(&page_id) = page_map.get(page_num) {
            page_ids_to_keep.push(page_id);
        }
    }

    if page_ids_to_keep.is_empty() {
        return Err(anyhow!("No valid pages found for extraction"));
    }

    // 4. 获取 Pages 字典，更新 Kids 和 Count
    let pages_dict = doc.get_dictionary(pages_id)?;
    let mut pages_dict = pages_dict.clone();

    // 更新 Kids - 只保留需要的页面引用
    let kids: Vec<Object> = page_ids_to_keep.iter().map(|&id| Object::Reference(id)).collect();

    pages_dict.set("Kids", Object::Array(kids));
    pages_dict.set("Count", page_ids_to_keep.len() as i64);

    // 替换 Pages 字典
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    // 5. 保存修改后的 PDF

    doc.renumber_objects();
    doc.save(output_path)?;

    Ok(())
}
