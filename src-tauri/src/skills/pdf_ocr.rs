/// PDF OCR Skill
/// 从 PDF 页面提取内嵌图片 → 图像解码 → Windows OCR 文字识别
/// 用于扫描件/图片型 PDF（无文本层）的文本提取
use std::path::Path;
use image::GenericImageView;
use lopdf::{Document, Object};

pub struct PdfOcr;

impl PdfOcr {
    /// 对整个 PDF 执行 OCR，返回每页识别文本
    pub fn ocr_document(file_path: &Path) -> Result<Vec<String>, String> {
        let doc = Document::load(file_path)
            .map_err(|e| format!("无法打开PDF文件: {}", e))?;

        let page_count = doc.page_iter().count();
        let mut page_texts = Vec::with_capacity(page_count);

        for (page_num, _object_id) in doc.page_iter() {
            match Self::extract_images_from_page(&doc, page_num) {
                Ok(images) => {
                    if images.is_empty() {
                        page_texts
                            .push(format!("[第{}页: 无可提取的内嵌图片]", page_num));
                    } else {
                        let mut page_text = String::new();
                        for (img_idx, img_bytes) in images.iter().enumerate() {
                            match Self::ocr_image_bytes(img_bytes) {
                                Ok(text) => {
                                    let trimmed = text.trim().to_string();
                                    if !trimmed.is_empty() {
                                        page_text.push_str(&trimmed);
                                        page_text.push('\n');
                                    }
                                }
                                Err(e) => {
                                    page_text.push_str(&format!(
                                        "[第{}页图像{} OCR失败: {}]\n",
                                        page_num,
                                        img_idx + 1,
                                        e
                                    ));
                                }
                            }
                        }
                        page_texts.push(if page_text.trim().is_empty() {
                            format!("[第{}页: 内嵌图片无可识别文字]", page_num)
                        } else {
                            format!("--- 第 {} 页 ---\n{}", page_num, page_text.trim())
                        });
                    }
                }
                Err(e) => {
                    page_texts.push(format!("[第{}页提取失败: {}]", page_num, e));
                }
            }
        }

        Ok(page_texts)
    }

    /// 从 PDF 指定页面提取所有内嵌图片的字节数据
    fn extract_images_from_page(doc: &Document, page_num: u32) -> Result<Vec<Vec<u8>>, String> {
        let mut images = Vec::new();

        // 获取页面对象
        let page_id = doc
            .page_iter()
            .find(|(num, _)| *num == page_num)
            .ok_or_else(|| format!("找不到第{}页", page_num))?;

        let page_dict = doc
            .get_dictionary(page_id)
            .map_err(|e| format!("读取第{}页字典失败: {}", page_num, e))?;

        // 遍历 Resources → XObject 链
        let resources = match page_dict.get(b"Resources") {
            Ok(r) => Self::resolve_to_dictionary(doc, r),
            Err(_) => return Ok(images),
        };

        let xobject = match resources
            .as_ref()
            .and_then(|r| r.get(b"XObject").ok())
        {
            Some(xo) => Self::resolve_to_dictionary(doc, xo),
            None => return Ok(images),
        };

        let xobject_dict = match xobject {
            Some(d) => d,
            None => return Ok(images),
        };

        // 遍历 XObject 条目，提取 Image 类型流
        for (_name, obj) in xobject_dict.iter() {
            let stream = match obj {
                Object::Stream(s) => Some(s),
                Object::Reference(id) => match doc.get_object(*id) {
                    Ok(Object::Stream(s)) => Some(s),
                    _ => None,
                },
                _ => None,
            };

            if let Some(stream) = stream {
                let subtype = stream
                    .dict
                    .get(b"Subtype")
                    .and_then(|o| o.as_name())
                    .unwrap_or(b"");

                if subtype != b"Image".as_slice() {
                    continue;
                }

                // 获取图像原始数据（尝试解压缩）
                let data = Self::extract_stream_data(stream)?;
                images.push(data);
            }
        }

        Ok(images)
    }

    /// 从 lopdf Stream 提取解压后的图像字节
    fn extract_stream_data(stream: &lopdf::Stream) -> Result<Vec<u8>, String> {
        let mut s = stream.clone();
        // lopdf 内置解压（处理 FlateDecode 等），DCTDecode(JPEG) 不解压
        // decompress() 就地修改流，无返回值（失败静默保留原始数据）
        s.decompress();
        Ok(s.content.clone())
    }

    /// 对图像字节执行 Windows OCR 识别
    fn ocr_image_bytes(img_bytes: &[u8]) -> Result<String, String> {
        // 用 image crate 解码
        let img = image::load_from_memory(img_bytes)
            .map_err(|e| format!("图像解码失败: {}", e))?;

        let (width, height) = img.dimensions();
        if width == 0 || height == 0 {
            return Err("图像尺寸为零".to_string());
        }

        // 转为 BGRA8 像素数据（Windows OCR 需要）
        let bgra = img.to_rgba8();
        let pixels: Vec<u8> = bgra
            .pixels()
            .flat_map(|p| [p[2], p[1], p[0], p[3]]) // RGBA → BGRA
            .collect();

        Self::windows_ocr(&pixels, width, height)
    }

    /// 调用 Windows 内置 OCR 引擎识别文字
    fn windows_ocr(pixels: &[u8], width: u32, height: u32) -> Result<String, String> {
        use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
        use windows::Media::Ocr::OcrEngine;
        use windows::Storage::Streams::DataWriter;

        // 创建 SoftwareBitmap
        let writer = DataWriter::new()
            .map_err(|e| format!("创建DataWriter失败: {}", e))?;
        writer
            .WriteBytes(pixels)
            .map_err(|e| format!("写入像素数据失败: {}", e))?;
        let buffer = writer
            .DetachBuffer()
            .map_err(|e| format!("分离缓冲区失败: {}", e))?;

        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            width as i32,
            height as i32,
        )
        .map_err(|e| format!("创建SoftwareBitmap失败: {}", e))?;

        // 创建 OCR 引擎（使用用户配置文件中的语言）
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()
            .map_err(|e| format!("创建OCR引擎失败: {}", e))?;

        // 执行识别（同步等待）
        let result = engine
            .RecognizeAsync(&bitmap)
            .map_err(|e| format!("启动OCR识别失败: {}", e))?
            .get()
            .map_err(|e| format!("OCR识别失败: {}", e))?;

        let text = result
            .Text()
            .map_err(|e| format!("获取OCR文本失败: {}", e))?;

        Ok(text.to_string())
    }

    /// 将 lopdf Object 解析为 Dictionary
    fn resolve_to_dictionary(
        doc: &Document,
        obj: &Object,
    ) -> Option<lopdf::Dictionary> {
        match obj {
            Object::Dictionary(d) => Some(d.clone()),
            Object::Reference(id) => match doc.get_object(*id) {
                Ok(Object::Dictionary(d)) => Some(d.clone()),
                Ok(Object::Stream(s)) => Some(s.dict.clone()),
                _ => None,
            },
            Object::Stream(s) => Some(s.dict.clone()),
            _ => None,
        }
    }
}
