use crate::models::{ScreenshotOcrRegion, ScreenshotOcrResult};
use image::{DynamicImage, GenericImageView};
use paddleocr_rs_onnx::{OcrEngine, OrderBy};
use std::{path::Path, sync::OnceLock};
use thiserror::Error;

const DETECTION_MODEL: &[u8] = include_bytes!("../resources/ocr/PP-OCRv5_mobile_det.onnx");
const RECOGNITION_MODEL: &[u8] = include_bytes!("../resources/ocr/PP-OCRv5_mobile_rec.onnx");
const CHARACTER_DICTIONARY: &[u8] = include_bytes!("../resources/ocr/ppocrv5_dict.txt");
const MIN_CONFIDENCE: f32 = 0.5;

static OCR_ENGINE: OnceLock<Result<OcrEngine, String>> = OnceLock::new();

#[derive(Debug, Error)]
pub enum ScreenshotOcrError {
    #[error("screenshot image is unavailable")]
    ImageUnavailable,
    #[error("OCR region is outside the screenshot")]
    InvalidRegion,
    #[error("screenshot image loading failed: {0}")]
    ImageLoading(#[source] image::ImageError),
    #[error("PaddleOCR initialization failed: {0}")]
    Initialization(String),
    #[error("PaddleOCR recognition failed: {0}")]
    Recognition(#[source] paddleocr_rs_onnx::PaddleOcrError),
}

impl ScreenshotOcrError {
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::ImageUnavailable => {
                "画像ファイルが見つからないため、文字起こしできませんでした。"
            }
            Self::InvalidRegion => "選択範囲が小さすぎるため、文字起こしできませんでした。",
            Self::ImageLoading(_) => {
                "画像を読み込めませんでした。ファイルが破損していないか確認してください。"
            }
            Self::Initialization(_) | Self::Recognition(_) => {
                "画像からテキストを文字起こしできませんでした。しばらくしてからもう一度お試しください。"
            }
        }
    }
}

pub fn recognize_japanese_text(
    path: &Path,
    region: Option<ScreenshotOcrRegion>,
) -> Result<ScreenshotOcrResult, ScreenshotOcrError> {
    if !path.is_file() {
        return Err(ScreenshotOcrError::ImageUnavailable);
    }

    let image = image::open(path).map_err(ScreenshotOcrError::ImageLoading)?;
    let image = crop_image(image, region)?;
    let engine = OCR_ENGINE
        .get_or_init(|| {
            OcrEngine::new(DETECTION_MODEL, RECOGNITION_MODEL, CHARACTER_DICTIONARY)
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| ScreenshotOcrError::Initialization(error.clone()))?;
    let blocks = engine
        .recognize_all(&image, OrderBy::Horizontal)
        .map_err(ScreenshotOcrError::Recognition)?;
    let text = blocks
        .into_iter()
        .filter(|block| block.confidence >= MIN_CONFIDENCE)
        .map(|block| normalize_text(&block.text))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(ScreenshotOcrResult { text })
}

fn crop_image(
    image: DynamicImage,
    region: Option<ScreenshotOcrRegion>,
) -> Result<DynamicImage, ScreenshotOcrError> {
    let Some(region) = region else {
        return Ok(image);
    };
    let (x, y, width, height) = normalized_crop(region, image.dimensions())?;
    Ok(image.crop_imm(x, y, width, height))
}

fn normalized_crop(
    region: ScreenshotOcrRegion,
    image_dimensions: (u32, u32),
) -> Result<(u32, u32, u32, u32), ScreenshotOcrError> {
    let values = [region.x, region.y, region.width, region.height];
    if values.iter().any(|value| !value.is_finite())
        || region.x < 0.0
        || region.y < 0.0
        || region.width <= 0.0
        || region.height <= 0.0
        || region.x + region.width > 1.0
        || region.y + region.height > 1.0
    {
        return Err(ScreenshotOcrError::InvalidRegion);
    }

    let (image_width, image_height) = image_dimensions;
    let left = (region.x * f64::from(image_width)).floor() as u32;
    let top = (region.y * f64::from(image_height)).floor() as u32;
    let right = ((region.x + region.width) * f64::from(image_width)).ceil() as u32;
    let bottom = ((region.y + region.height) * f64::from(image_height)).ceil() as u32;
    let right = right.min(image_width);
    let bottom = bottom.min(image_height);
    let width = right.saturating_sub(left);
    let height = bottom.saturating_sub(top);
    if width < 2 || height < 2 {
        return Err(ScreenshotOcrError::InvalidRegion);
    }

    Ok((left, top, width, height))
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{ScreenshotOcrError, normalize_text, normalized_crop, recognize_japanese_text};
    use crate::models::ScreenshotOcrRegion;
    use std::path::Path;

    #[test]
    fn normalizes_line_endings_and_outer_whitespace() {
        assert_eq!(normalize_text("  一行目\r\n二行目\r\n  "), "一行目\n二行目");
    }

    #[test]
    fn converts_a_normalized_region_to_covering_pixels() {
        let region = ScreenshotOcrRegion {
            x: 0.1,
            y: 0.2,
            width: 0.25,
            height: 0.5,
        };

        assert_eq!(
            normalized_crop(region, (100, 50)).unwrap(),
            (10, 10, 25, 25)
        );
    }

    #[test]
    fn rejects_regions_outside_the_image() {
        let region = ScreenshotOcrRegion {
            x: 0.8,
            y: 0.2,
            width: 0.3,
            height: 0.5,
        };

        assert!(matches!(
            normalized_crop(region, (100, 50)),
            Err(ScreenshotOcrError::InvalidRegion)
        ));
    }

    #[test]
    fn reports_an_unavailable_screenshot_before_starting_ocr() {
        let error = recognize_japanese_text(Path::new("missing-screenshot.png"), None).unwrap_err();
        assert!(matches!(error, ScreenshotOcrError::ImageUnavailable));
        assert_eq!(
            error.user_message(),
            "画像ファイルが見つからないため、文字起こしできませんでした。"
        );
    }

    #[test]
    #[ignore = "manual OCR model smoke test"]
    fn recognizes_japanese_in_a_repository_screenshot() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/images/library.png");
        let result = recognize_japanese_text(&path, None).unwrap();

        println!("{}", result.text);
        assert!(result.text.contains("ライブラリ"), "{}", result.text);
    }
}
