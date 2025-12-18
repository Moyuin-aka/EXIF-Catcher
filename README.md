# EXIF Catcher

📸 静态相册生成工具 - 批量提取照片EXIF信息并转换为WebP格式

专为摄影博客设计的照片处理工具，可以快速处理大量照片，生成结构化的元数据文件，完美集成到静态网站生成器（Astro、Hugo、Next.js等）。

## 功能特性

- 并行处理多张照片，充分利用多核CPU性能
- 提取完整的EXIF元数据（相机型号、ISO、光圈、快门等）
- 有损WebP压缩，大幅减小文件体积（通常压缩率80%以上）
- 生成JSON格式的元数据文件，便于静态博客集成
- 结构化输出，支持多相册管理
- 交互式CLI界面，操作简单

## 安装

### 方式一：下载预编译版本（推荐）

从 [Releases](https://github.com/Moyuin-aka/EXIF-Catcher/releases) 页面下载对应平台的二进制文件：

- **macOS**: `exif-catcher-macos-aarch64` (Apple Silicon) 或 `exif-catcher-macos-x86_64` (Intel)
- **Linux**: `exif-catcher-linux-x86_64`
- **Windows**: `exif-catcher-windows-x86_64.exe`

解压后即可使用，无需安装 Rust 环境。

```bash
# macOS / Linux
tar -xzf exif-catcher-*.tar.gz
chmod +x exif-catcher
./exif-catcher --help

# 可选：移动到系统路径
sudo mv exif-catcher /usr/local/bin/
```

### 方式二：使用 Cargo 安装

如果已安装 Rust 环境（需要 1.75+）：

```bash
cargo install --git https://github.com/Moyuin-aka/EXIF-Catcher
```

### 方式三：从源码构建

```bash
git clone https://github.com/Moyuin-aka/EXIF-Catcher
cd EXIF-Catcher
cargo build --release
```

编译后的二进制文件位于 `target/release/exif-catcher`

## 使用方法

### 交互式模式（推荐新手）

```bash
exif-catcher
```

程序会引导你输入必要的参数：
- 图片目录路径
- 输出目录
- 是否转换WebP
- WebP质量设置

### 命令行模式

```bash
# 基本使用
exif-catcher -i /path/to/photos -o dist -q 80

# 仅提取EXIF，不转换图片
exif-catcher -i /path/to/photos --skip-webp

# 查看帮助
exif-catcher --help
```

### 参数说明

| 参数 | 简写 | 说明 | 默认值 |
|------|------|------|--------|
| `--input` | `-i` | 输入目录（包含原始图片） | 必需 |
| `--output` | `-o` | 输出根目录 | `dist` |
| `--quality` | `-q` | WebP质量 (1-100) | `80` |
| `--max-width` | - | 限制图片最大宽度（像素），0为不限制 | `0` |
| `--skip-webp` | - | 跳过WebP转换 | `false` |
| `--yes` | `-y` | 跳过交互确认 | `false` |

## 输出结构

```
dist/
└── 相册名称/
    ├── img/
    │   ├── photo1.webp
    │   ├── photo2.webp
    │   └── ...
    └── exif.json
```

### exif.json 格式

```json
[
  {
    "original": "IMG_0001.jpg",
    "webp": "IMG_0001.webp",
    "original_size": 7425632,
    "webp_size": 716800,
    "make": "Canon",
    "model": "Canon EOS R5",
    "lens": "RF24-70mm F2.8 L IS USM",
    "iso": "400",
    "shutter_speed": "1/1000",
    "aperture": "2.8",
    "focal_length": "50 mm",
    "date_time": "2024-12-18 14:30:00",
    "width": 8192,
    "height": 5464
  }
]
```

## 使用示例

### 为静态博客生成相册

```bash
# 1. 处理照片
exif-catcher -i ~/Photos/Travel/Paris -o ~/blog/static/galleries

# 2. 上传到云存储（使用Rclone）
rclone sync ~/blog/static/galleries/Paris r2:my-bucket/galleries/Paris
```

### 批量处理多个相册

```bash
#!/bin/bash
for album in ~/Photos/Albums/*; do
  exif-catcher -i "$album" -o dist -q 85
done
```

## 支持的图片格式

- ✅ JPEG (.jpg, .jpeg)
- ✅ PNG (.png)  
- ✅ HEIC (.heic)
- ✅ WebP (.webp)

## 性能优化建议

- **质量设置**: 
  - 75-80: 高质量，适合专业摄影展示
  - 85-90: 接近无损，文件略大
  - 60-70: 较小文件，适合网页快速加载
- **图片尺寸优化**: 
  - 使用 `--max-width 2048` 可以大幅提升处理速度（3-5倍）
  - 对于网页展示，2048px 宽度已经足够清晰
  - 原始4K图片（4096px）处理较慢，建议缩放

- **并行处理**: 工具自动利用所有CPU核心，无需手动配置

- **内存占用**: 处理大图时（如>30MP）内存占用可能达到几GB，建议预留充足内存

## 常见问题

### Q: 某些相机的EXIF信息缺失？
A: 不同相机厂商使用的EXIF标签略有差异，如发现问题请提 [Issue](https://github.com/Moyuin-aka/EXIF-Catcher/issues)。

### Q: 没有Rust环境可以使用吗？
A: 可以！直接从 [Releases](https://github.com/Moyuin-aka/EXIF-Catcher/releases) 下载预编译的二进制文件。

### Q: 处理速度慢怎么办？
A: 使用 `--max-width 2048` 参数限制图片宽度，可以提速3-5倍。
### Q: 如何集成到Astro/Next.js等静态网站？
A: 生成的JSON可直接导入：
```javascript
import photos from './galleries/paris/exif.json';
```

## 开发

```bash
# 运行开发版本
cargo run -- -i test-photos

# 运行测试
cargo test

# 代码格式化
cargo fmt

# 代码检查
cargo clippy
```

## 技术栈

- [clap](https://github.com/clap-rs/clap) - CLI参数解析
- [rayon](https://github.com/rayon-rs/rayon) - 并行处理
- [kamadak-exif](https://github.com/kamadak/exif-rs) - EXIF解析
- [image](https://github.com/image-rs/image) - 图片处理
- [webp](https://github.com/jaredforth/webp) - WebP编码

## 许可证

MIT License

## 贡献

欢迎提交Issue和Pull Request！