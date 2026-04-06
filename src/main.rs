use clap::Parser;
use colored::*;
use dialoguer::{Input, Confirm};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Cursor;
use anyhow::{Context, Result};
use rayon::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use walkdir::WalkDir;

// Windows 平台的暂停功能（双击 exe 时不会闪退）
#[cfg(windows)]
fn pause_on_windows() {
    use std::io::{self, Write};
    println!("\n{}", "▶️  按任意键退出...".bright_black());
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
}

#[cfg(not(windows))]
fn pause_on_windows() {
    // 非 Windows 平台不需要暂停
}

// CLI定义
#[derive(Parser)]
#[command(author, version, about = "静态相册生成器 - EXIF提取 + WebP转换")]
struct Cli {
    #[arg(short, long, value_name = "DIR")]
    input: Option<PathBuf>,
    
    #[arg(short, long, value_name = "DIR", default_value = "dist")]
    output: PathBuf,

    /// 就地输出：直接写入输入目录（不再嵌套 output/相册名）
    #[arg(long)]
    in_place: bool,

    /// 清理原图：仅删除已成功转换为 WebP 的原始图片
    #[arg(long)]
    cleanup: bool,
    
    #[arg(long)]
    skip_webp: bool,
    
    #[arg(short, long, default_value = "80")]
    quality: u8,
    
    /// 限制最大宽度（像素），0为不限制
    #[arg(long, default_value = "0")]
    max_width: u32,
    
    /// 并发处理的线程数（默认为CPU核心数）
    #[arg(long)]
    jobs: Option<usize>,
    
    /// 递归处理子目录
    #[arg(short, long)]
    recursive: bool,
    
    #[arg(short = 'y', long)]
    yes: bool,
}

// 数据结构
#[derive(Serialize, Deserialize, Debug)]
struct Photo {
    original: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    webp: Option<String>,
    original_size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    webp_size: Option<u64>,
    #[serde(flatten)]
    exif: ExifData,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExifData {
    #[serde(skip_serializing_if = "Option::is_none")]
    make: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lens: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iso: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shutter_speed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aperture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focal_length: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    date_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
}

// 提取EXIF
fn extract_exif(path: &Path) -> Result<ExifData> {
    let file_data = fs::read(path)?;
    let mut cursor = Cursor::new(file_data);
    let exif_reader = exif::Reader::new();
    
    let exif = match exif_reader.read_from_container(&mut cursor) {
        Ok(v) => v,
        Err(_) => {
            // 没有 EXIF 或读取失败（PNG/WebP 常见），返回空值而非错误
            return Ok(ExifData {
                make: None,
                model: None,
                lens: None,
                iso: None,
                shutter_speed: None,
                aperture: None,
                focal_length: None,
                date_time: None,
                width: None,
                height: None,
            });
        }
    };
    
    let get_field = |tag: exif::Tag| -> Option<String> {
        if let Some(field) = exif.get_field(tag, exif::In::PRIMARY) {
            return Some(field.display_value().to_string());
        }
        exif.fields()
            .find(|f| f.tag == tag)
            .map(|field| field.display_value().to_string())
    };
    
    Ok(ExifData {
        make: get_field(exif::Tag::Make).map(|s| s.trim_matches('"').to_string()),
        model: get_field(exif::Tag::Model).map(|s| s.trim_matches('"').to_string()),
        lens: get_field(exif::Tag::LensModel).map(|s| s.trim_matches('"').to_string()),
        iso: get_field(exif::Tag::PhotographicSensitivity)
            .or_else(|| get_field(exif::Tag::ISOSpeed)),
        shutter_speed: get_field(exif::Tag::ExposureTime)
            .or_else(|| get_field(exif::Tag::ShutterSpeedValue)),
        aperture: get_field(exif::Tag::FNumber)
            .or_else(|| get_field(exif::Tag::ApertureValue)),
        focal_length: get_field(exif::Tag::FocalLength),
        date_time: get_field(exif::Tag::DateTime),
        width: get_field(exif::Tag::PixelXDimension)
            .or_else(|| get_field(exif::Tag::ImageWidth))
            .and_then(|s| s.parse().ok()),
        height: get_field(exif::Tag::PixelYDimension)
            .or_else(|| get_field(exif::Tag::ImageLength))
            .and_then(|s| s.parse().ok()),
    })
}

// 转换WebP（有损压缩）
fn convert_to_webp(input_path: &Path, output_path: &Path, quality: u8, max_width: u32) -> Result<u64> {
    use image::DynamicImage;
    
    // 读取图片
    let mut img = image::open(input_path)?;
    let output_with_ext = output_path.with_extension("webp");
    
    // 如果设置了最大宽度，调整图片大小
    if max_width > 0 && img.width() > max_width {
        let ratio = max_width as f32 / img.width() as f32;
        let new_height = (img.height() as f32 * ratio) as u32;
        img = img.resize(max_width, new_height, image::imageops::FilterType::Lanczos3);
    }
    
    // 转换为RGB8格式（WebP需要）
    let rgb_img = match img {
        DynamicImage::ImageRgb8(rgb) => rgb,
        _ => img.to_rgb8(),
    };
    
    // 使用webp库进行有损编码
    let encoder = webp::Encoder::from_rgb(
        rgb_img.as_raw(),
        rgb_img.width(),
        rgb_img.height(),
    );
    
    // 设置质量（0-100）
    let webp_data = encoder.encode(quality as f32);
    
    // 保存文件
    fs::write(&output_with_ext, &*webp_data)?;
    
    Ok(fs::metadata(&output_with_ext)?.len())
}

// 处理单张图片
fn process_image(
    path: &Path,
    output_img_dir: &Path,
    skip_webp: bool,
    quality: u8,
    max_width: u32,
) -> Result<Photo> {
    let filename = path.file_name().unwrap().to_string_lossy().to_string();
    let original_size = fs::metadata(path)?.len();
    let exif = extract_exif(path)?;
    
    // 检查输入格式
    let is_webp_input = path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.eq_ignore_ascii_case("webp"))
        .unwrap_or(false);
    
    let is_heic = path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.eq_ignore_ascii_case("heic") || s.eq_ignore_ascii_case("heif"))
        .unwrap_or(false);
    
    // WebP 转换：跳过已是 WebP 的和 HEIC 格式（image crate 可能不支持）
    let (webp_filename, webp_size) = if !skip_webp && !is_webp_input && !is_heic {
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let webp_name = format!("{}.webp", stem);
        let webp_path = output_img_dir.join(&webp_name);
        
        // WebP 转换失败不致命，降级为“无 WebP”
        match convert_to_webp(path, &webp_path, quality, max_width) {
            Ok(size) => (Some(webp_name), Some(size)),
            Err(_) => (None, None), // 转换失败，但仍然输出 EXIF
        }
    } else {
        (None, None)
    };
    
    Ok(Photo {
        original: filename,
        webp: webp_filename,
        original_size,
        webp_size,
        exif,
    })
}

// 扫描图片
// 扫描图片（返回 (file_path, relative_dir)）
fn scan_images(dir: &Path, recursive: bool) -> Result<Vec<(PathBuf, String)>> {
    let mut images = Vec::new();
    
    if recursive {
        // 递归扫描所有子目录
        for entry in WalkDir::new(dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if matches!(ext_str.as_str(), "jpg" | "jpeg" | "png" | "heic" | "webp") {
                        // 计算相对于输入目录的相对路径
                        let parent = path.parent().unwrap();
                        let rel_dir = parent.strip_prefix(dir)
                            .unwrap_or(Path::new(""))
                            .to_string_lossy()
                            .to_string();
                        images.push((path.to_path_buf(), rel_dir));
                    }
                }
            }
        }
    } else {
        // 只扫描当前目录
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if matches!(ext_str.as_str(), "jpg" | "jpeg" | "png" | "heic" | "webp") {
                        images.push((path, String::new()));
                    }
                }
            }
        }
    }
    images.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(images)
}

fn build_output_dir(
    input_dir: &Path,
    output_root: &Path,
    folder_name: &str,
    rel_dir: &str,
    in_place: bool,
) -> PathBuf {
    if in_place {
        if rel_dir.is_empty() {
            input_dir.to_path_buf()
        } else {
            input_dir.join(rel_dir)
        }
    } else if rel_dir.is_empty() {
        output_root.join(folder_name)
    } else {
        output_root.join(folder_name).join(rel_dir)
    }
}

fn should_cleanup_original(path: &Path, photo: &Photo) -> bool {
    let is_original_image = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "heic" | "heif"
            )
        })
        .unwrap_or(false);

    is_original_image && photo.webp.is_some()
}

fn resolve_folder_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .or_else(|| {
            fs::canonicalize(path)
                .ok()
                .and_then(|p| p.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()))
        })
        .unwrap_or_else(|| "album".to_string())
}

// 批量处理
fn process_directory(
    input_dir: &Path,
    output_root: &Path,
    skip_webp: bool,
    quality: u8,
    max_width: u32,
    recursive: bool,
    in_place: bool,
    cleanup: bool,
) -> Result<()> {
    println!("\n{}", "🔍 扫描图片...".cyan().bold());
    if recursive {
        println!("📂 递归模式: 将扫描所有子目录\n");
    }
    
    let image_files = scan_images(input_dir, recursive)?;
    if image_files.is_empty() {
        println!("{}", "⚠️  没有找到图片！".yellow());
        return Ok(());
    }
    
    println!("📂 找到 {} 张图片\n", image_files.len());
    
    // 按相对目录分组
    let mut groups: std::collections::HashMap<String, Vec<PathBuf>> = std::collections::HashMap::new();
    for (path, rel_dir) in &image_files {
        groups.entry(rel_dir.clone()).or_insert_with(Vec::new).push(path.clone());
    }
    
    let folder_name = resolve_folder_name(input_dir);

    if in_place {
        println!("📁 输出结构:");
        if recursive && groups.len() > 1 {
            println!("  {}", input_dir.display().to_string().cyan());
            println!("  └── 每个目录下将生成 img/ 与 exif.json\n");
        } else {
            println!("  {}", input_dir.display().to_string().cyan());
            println!("  ├── img/");
            println!("  └── exif.json\n");
        }
    } else if recursive && groups.len() > 1 {
        println!("📁 目录结构:");
        println!("  {}/ (输出根目录)", output_root.display());
        for rel_dir in groups.keys() {
            let display_dir = if rel_dir.is_empty() {
                folder_name.clone()
            } else {
                format!("{}/{}", folder_name, rel_dir)
            };
            println!("    ├── {}/", display_dir);
            println!("    │   ├── img/");
            println!("    │   └── exif.json");
        }
        println!();
    } else {
        let output_dir = output_root.join(&folder_name);
        println!("📁 输出结构:");
        println!("  {}", output_dir.display().to_string().cyan());
        println!("  ├── img/");
        println!("  └── exif.json\n");
    }
    
    let pb = ProgressBar::new(image_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("█▓▒░ "),
    );
    
    if !skip_webp {
        if max_width > 0 {
            println!("⚡ 性能优化: 图片将缩放至最大宽度 {}px\n", max_width);
        } else {
            println!("💡 提示: 使用 --max-width 2048 可以加快处理速度\n");
        }
    }
    
    // 按目录分组处理
    let mut all_photos_by_dir: std::collections::HashMap<String, Vec<Photo>> = std::collections::HashMap::new();
    let mut total_errors = Vec::new();
    let mut cleanup_candidates = Vec::new();
    
    for (rel_dir, files) in groups {
        let output_dir = build_output_dir(input_dir, output_root, &folder_name, &rel_dir, in_place);
        let output_img_dir = output_dir.join("img");
        fs::create_dir_all(&output_img_dir)?;
        
        let results: Vec<Result<Photo>> = files
            .par_iter()
            .map(|path| {
                let result = process_image(path, &output_img_dir, skip_webp, quality, max_width);
                pb.inc(1);
                result
            })
            .collect();
        
        let mut photos = Vec::new();
        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok(photo) => {
                    if cleanup && should_cleanup_original(&files[i], &photo) {
                        cleanup_candidates.push(files[i].clone());
                    }
                    photos.push(photo)
                }
                Err(e) => total_errors.push((files[i].clone(), e)),
            }
        }
        
        all_photos_by_dir.insert(rel_dir.clone(), photos);
    }
    
    pb.finish_with_message("完成!");
    
    // 统计总结果
    let total_success: usize = all_photos_by_dir.values().map(|v| v.len()).sum();
    
    println!("\n{}", "📊 处理结果:".bold());
    println!("  ✅ 成功: {}", total_success.to_string().green().bold());
    if !total_errors.is_empty() {
        println!("  ❌ 失败: {}", total_errors.len().to_string().red().bold());
    }
    
    // 计算压缩统计
    if !skip_webp && total_success > 0 {
        let total_orig: u64 = all_photos_by_dir.values()
            .flat_map(|v| v.iter())
            .map(|p| p.original_size)
            .sum();
        let total_webp: u64 = all_photos_by_dir.values()
            .flat_map(|v| v.iter())
            .filter_map(|p| p.webp_size)
            .sum();
        let ratio = (total_webp as f64 / total_orig as f64) * 100.0;
        
        println!("\n{}", "💾 压缩统计:".bold());
        println!("  原始: {:.2} MB", total_orig as f64 / 1_048_576.0);
        println!("  WebP: {:.2} MB", total_webp as f64 / 1_048_576.0);
        println!("  压缩率: {:.1}%", ratio);
    }
    
    // 保存 JSON 文件
    if total_success > 0 {
        println!("\n{}", "✨ 完成!".green().bold());
        for (rel_dir, photos) in all_photos_by_dir {
            if photos.is_empty() {
                continue;
            }
            
            let output_dir = build_output_dir(input_dir, output_root, &folder_name, &rel_dir, in_place);
            let output_json = output_dir.join("exif.json");
            let output_img_dir = output_dir.join("img");
            
            let json = serde_json::to_string_pretty(&photos)?;
            fs::write(&output_json, json)?;
            
            println!("📝 EXIF: {}", output_json.display().to_string().cyan());
            if !skip_webp {
                println!("🖼️  图片: {}", output_img_dir.display().to_string().cyan());
            }
        }

        if cleanup {
            cleanup_candidates.sort();
            cleanup_candidates.dedup();

            let mut deleted = 0usize;
            let mut cleanup_errors = Vec::new();
            for path in cleanup_candidates {
                match fs::remove_file(&path) {
                    Ok(_) => deleted += 1,
                    Err(e) => cleanup_errors.push((path, e)),
                }
            }

            println!("\n{}", "🧹 清理结果:".bold());
            println!("  🗑️  删除原图: {}", deleted.to_string().green().bold());
            if !cleanup_errors.is_empty() {
                println!("  ❌ 删除失败: {}", cleanup_errors.len().to_string().red().bold());
                for (path, e) in cleanup_errors.iter().take(3) {
                    eprintln!("     - {}: {}", path.display(), e);
                }
                if cleanup_errors.len() > 3 {
                    eprintln!("     ... 还有 {} 个错误", cleanup_errors.len() - 3);
                }
            }
        }

        if in_place {
            println!("\n{}", "💡 提示: 现在可以上传当前目录下的 img/ 与 exif.json 了!".bright_black());
        } else {
            println!("\n{}", "💡 提示: 现在可以用 Rclone 上传 dist/ 文件夹了!".bright_black());
        }
    }
    
    Ok(())
}

// 交互模式
fn interactive_mode() -> Result<(PathBuf, PathBuf, bool, u8, u32, bool, bool, bool)> {
    println!("{}", "
======================================
  📸 EXIF Catcher
  静态相册生成器
======================================
".cyan().bold());
    
    let input: String = Input::new()
        .with_prompt("📂 图片目录")
        .default("./photos".to_string())
        .interact_text()?;
    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        anyhow::bail!("目录不存在");
    }
    
    let output: String = Input::new()
        .with_prompt("💾 输出目录")
        .default("./dist".to_string())
        .interact_text()?;
    let output_path = PathBuf::from(&output);
    
    let recursive = Confirm::new()
        .with_prompt("📂 递归处理子目录?")
        .default(false)
        .interact()?;

    let in_place = Confirm::new()
        .with_prompt("📍 就地输出（写回输入目录）?")
        .default(false)
        .interact()?;
    
    let skip_webp = !Confirm::new()
        .with_prompt("🎨 转换为WebP?")
        .default(true)
        .interact()?;
    
    let (quality, max_width) = if !skip_webp {
        let q: String = Input::new()
            .with_prompt("🎚️  质量 (1-100)")
            .default("80".to_string())
            .interact_text()?;
        
        let resize = Confirm::new()
            .with_prompt("⚡ 限制图片宽度以加快处理?")
            .default(false)
            .interact()?;
        
        let max_w = if resize {
            let w: String = Input::new()
                .with_prompt("📏 最大宽度 (像素)")
                .default("2048".to_string())
                .interact_text()?;
            w.parse().unwrap_or(2048)
        } else {
            0
        };
        
        (q.parse().unwrap_or(80), max_w)
    } else {
        (80, 0)
    };

    let cleanup = if !skip_webp {
        Confirm::new()
            .with_prompt("🧹 转换成功后删除原图?")
            .default(false)
            .interact()?
    } else {
        false
    };
    
    let folder_name = resolve_folder_name(&input_path);
    println!("\n{}", "📋 配置:".bold());
    println!("  输入: {}", input_path.display().to_string().green());
    if in_place {
        println!("  输出: {}", "输入目录（就地）".green());
    } else {
        println!("  输出: {}/{}", output_path.display().to_string().green(), folder_name);
    }
    println!("  递归: {}", if recursive { "是".green() } else { "否".bright_black() });
    println!("  清理原图: {}", if cleanup { "是".yellow() } else { "否".bright_black() });
    if !skip_webp {
        println!("  WebP: 是 (质量: {})", quality);
    }
    
    if !Confirm::new().with_prompt("\n开始?").default(true).interact()? {
        anyhow::bail!("取消");
    }
    
    Ok((input_path, output_path, skip_webp, quality, max_width, recursive, in_place, cleanup))
}

// 主程序
fn main() {
    let result = run();
    
    // Windows 平台：如果出错或正常结束，都暂停一下让用户看到结果
    #[cfg(windows)]
    {
        if let Err(e) = &result {
            eprintln!("\n{} {}", "❌ 错误:".red().bold(), e);
        }
        pause_on_windows();
    }
    
    // 非 Windows 平台：直接退出
    #[cfg(not(windows))]
    {
        if let Err(e) = result {
            eprintln!("\n{} {}", "❌ 错误:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let has_any_cli_args = std::env::args_os().len() > 1;
    
    // 设置线程池大小（如果指定）
    if let Some(n) = cli.jobs {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()
            .context("创建线程池失败")?;
    }
    
    let (input_dir, output_root, skip_webp, quality, max_width, recursive, in_place, cleanup) = if let Some(input) = cli.input {
        if !input.exists() {
            anyhow::bail!("目录不存在");
        }
        // 确保 quality 在有效范围 1-100
        let quality = cli.quality.clamp(1, 100);
        (input, cli.output, cli.skip_webp, quality, cli.max_width, cli.recursive, cli.in_place, cli.cleanup)
    } else if has_any_cli_args || cli.yes {
        let input = PathBuf::from(".");
        if !input.exists() {
            anyhow::bail!("当前目录不存在");
        }
        // 命令行模式且未显式指定 -i 时，默认读取当前目录
        let quality = cli.quality.clamp(1, 100);
        (input, cli.output, cli.skip_webp, quality, cli.max_width, cli.recursive, cli.in_place, cli.cleanup)
    } else {
        interactive_mode()?
    };

    if cleanup && skip_webp {
        println!("{}", "⚠️  当前启用了 --skip-webp，--cleanup 不会删除任何文件。".yellow());
    }
    
    process_directory(
        &input_dir,
        &output_root,
        skip_webp,
        quality,
        max_width,
        recursive,
        in_place,
        cleanup,
    )?;
    Ok(())
}
