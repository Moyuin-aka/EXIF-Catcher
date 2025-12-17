use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use tower_http::cors::CorsLayer;
use tracing::{info, error};

// ============== 数据结构定义 ==============

/// API请求参数 - 接收图片URL
#[derive(Deserialize)]
struct ExifRequest {
    /// 图片的URL地址（S3或其他公开URL）
    url: String,
}

/// EXIF信息响应结构 - 返回给前端的数据
#[derive(Serialize)]
struct ExifResponse {
    /// 是否成功
    success: bool,
    /// 错误信息（如果有）
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    /// EXIF数据
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<ExifData>,
}

/// 详细的EXIF数据
#[derive(Serialize)]
struct ExifData {
    /// 相机制造商（如：Canon, Nikon, Sony）
    #[serde(skip_serializing_if = "Option::is_none")]
    make: Option<String>,
    /// 相机型号（如：Canon EOS R5）
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    /// 镜头型号
    #[serde(skip_serializing_if = "Option::is_none")]
    lens: Option<String>,
    /// ISO感光度
    #[serde(skip_serializing_if = "Option::is_none")]
    iso: Option<String>,
    /// 快门速度（如：1/1000）
    #[serde(skip_serializing_if = "Option::is_none")]
    shutter_speed: Option<String>,
    /// 光圈值（如：f/2.8）
    #[serde(skip_serializing_if = "Option::is_none")]
    aperture: Option<String>,
    /// 焦距（如：50mm）
    #[serde(skip_serializing_if = "Option::is_none")]
    focal_length: Option<String>,
    /// 拍摄日期时间
    #[serde(skip_serializing_if = "Option::is_none")]
    date_time: Option<String>,
    /// 图片宽度
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<String>,
    /// 图片高度
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<String>,
}

// ============== 核心功能实现 ==============

/// 从URL下载图片
async fn download_image(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    info!("开始下载图片: {}", url);
    
    // 发送HTTP GET请求（类似Python的 requests.get()）
    let response = reqwest::get(url).await?;
    
    // 检查HTTP状态码
    if !response.status().is_success() {
        return Err(format!("下载失败，状态码: {}", response.status()).into());
    }
    
    // 读取响应体为字节数组（类似Python的 response.content）
    let bytes = response.bytes().await?;
    info!("图片下载完成，大小: {} 字节", bytes.len());
    
    Ok(bytes.to_vec())
}

/// 解析图片的EXIF数据
fn parse_exif(image_data: &[u8]) -> Result<ExifData, Box<dyn std::error::Error>> {
    info!("开始解析EXIF数据");
    
    // 创建一个内存读取器（类似Python的 io.BytesIO）
    let cursor = Cursor::new(image_data);
    
    // 解析EXIF
    let exif_reader = exif::Reader::new();
    let exif = exif_reader.read_from_container(&mut cursor.clone())?;
    
    // 辅助函数：安全获取EXIF字段
    let get_field = |tag: exif::Tag| -> Option<String> {
        exif.get_field(tag, exif::In::PRIMARY)
            .map(|field| field.display_value().to_string())
    };
    
    // 提取各项EXIF信息
    let make = get_field(exif::Tag::Make);
    let model = get_field(exif::Tag::Model);
    let lens = get_field(exif::Tag::LensModel);
    let iso = get_field(exif::Tag::PhotographicSensitivity);
    let shutter_speed = get_field(exif::Tag::ExposureTime);
    let aperture = get_field(exif::Tag::FNumber);
    let focal_length = get_field(exif::Tag::FocalLength);
    let date_time = get_field(exif::Tag::DateTime);
    let width = get_field(exif::Tag::PixelXDimension);
    let height = get_field(exif::Tag::PixelYDimension);
    
    info!("EXIF解析成功");
    
    Ok(ExifData {
        make,
        model,
        lens,
        iso,
        shutter_speed,
        aperture,
        focal_length,
        date_time,
        width,
        height,
    })
}

// ============== API路由处理 ==============

/// 健康检查端点
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "exif_catcher",
        "version": "0.1.0"
    }))
}

/// 主要的EXIF提取端点
/// 
/// 使用方式：GET /exif?url=https://your-s3-bucket.com/image.jpg
async fn get_exif(Query(params): Query<ExifRequest>) -> impl IntoResponse {
    info!("收到EXIF请求: {}", params.url);
    
    // 下载图片
    let image_data = match download_image(&params.url).await {
        Ok(data) => data,
        Err(e) => {
            error!("图片下载失败: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(ExifResponse {
                    success: false,
                    error: Some(format!("无法下载图片: {}", e)),
                    data: None,
                }),
            );
        }
    };
    
    // 解析EXIF
    let exif_data = match parse_exif(&image_data) {
        Ok(data) => data,
        Err(e) => {
            error!("EXIF解析失败: {}", e);
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ExifResponse {
                    success: false,
                    error: Some(format!("无法解析EXIF数据: {}", e)),
                    data: None,
                }),
            );
        }
    };
    
    info!("EXIF数据提取成功");
    
    (
        StatusCode::OK,
        Json(ExifResponse {
            success: true,
            error: None,
            data: Some(exif_data),
        }),
    )
}

// ============== 主程序入口 ==============

#[tokio::main]
async fn main() {
    // 初始化日志系统
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    info!("🚀 启动 EXIF Catcher 服务");
    
    // 构建应用路由（类似Python Flask的 @app.route）
    let app = Router::new()
        .route("/", get(health_check))           // 健康检查
        .route("/health", get(health_check))     // 健康检查
        .route("/exif", get(get_exif))           // EXIF提取API
        .layer(CorsLayer::permissive());         // 启用CORS，允许跨域请求
    
    // 绑定端口
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    info!("📡 服务运行在: http://{}", addr);
    info!("📖 API文档:");
    info!("   - GET /health         - 健康检查");
    info!("   - GET /exif?url=...   - 获取EXIF数据");
    
    // 启动服务器
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("无法绑定端口");
    
    axum::serve(listener, app)
        .await
        .expect("服务器启动失败");
}
