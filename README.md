# EXIF Catcher

静态相册生成工具 - 批量提取照片EXIF信息并转换为WebP格式

## 功能特性

- 并行处理多张照片，充分利用多核CPU性能
- 提取完整的EXIF元数据（相机型号、ISO、光圈、快门等）
- 有损WebP压缩，大幅减小文件体积（通常压缩率80%以上）
- 生成JSON格式的元数据文件，便于静态博客集成
- 结构化输出，支持多相册管理
- 交互式CLI界面，操作简单

## 安装

### 前置要求

- Rust 1.75 或更高版本

### 从源码构建

```bash
git clone https://github.com/your-username/exif_catcher.git
cd exif_catcher
cargo build --release
```

编译后的二进制文件位于 `target/release/exif_catcher`

## 使用方法

### 交互式模式

```bash
cargo run
# 或
./target/release/exif_catcher
```

程序会引导你输入必要的参数：
- 图片目录路径
- 输出目录
- 是否转换WebP
- WebP质量设置

### 命令行模式

```bash
# 基本使用
exif_catcher -i /path/to/photos -o dist -q 80

# 仅提取EXIF，不转换图片
exif_catcher -i /path/to/photos --skip-webp

# 查看帮助
exif_catcher --help
```

### 参数说明

| 参数 | 简写 | 说明 | 默认值 |
|------|------|------|--------|
| `--input` | `-i` | 输入目录（包含原始图片） | 必需 |
| `--output` | `-o` | 输出根目录 | `dist` |
| `--quality` | `-q` | WebP质量 (1-100) | `80` |
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
exif_catcher -i ~/Photos/Travel/Paris -o ~/blog/static/galleries

# 2. 上传到云存储（使用Rclone）
rclone sync ~/blog/static/galleries/Paris r2:my-bucket/galleries/Paris
```

### 批量处理多个相册

```bash
#!/bin/bash
for album in ~/Photos/Albums/*; do
  exif_catcher -i "$album" -o dist -q 85
done
```

## 支持的图片格式

- JPEG (.jpg, .jpeg)
- PNG (.png)
- HEIC (.heic)
- WebP (.webp)

## 性能优化建议

- **质量设置**: 
  - 75-80: 高质量，适合专业摄影展示
  - 85-90: 接近无损，文件略大
  - 60-70: 较小文件，适合网页快速加载

- **并行处理**: 工具自动利用所有CPU核心，无需手动配置

- **内存占用**: 处理大图时（如>30MP）内存占用可能达到几GB，建议预留充足内存

## 常见问题

### Q: WebP压缩后文件反而变大？
A: v0.2.0版本已修复，现在使用真正的有损压缩。如遇此问题请更新到最新版本。

### Q: 某些相机的EXIF信息缺失？
A: 不同相机厂商使用的EXIF标签略有差异，如发现问题请提Issue。

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

## 更新日志

### v0.2.0
- 修复WebP压缩问题，实现真正的有损压缩
- 新增并行处理支持
- 优化输出结构
- 改进EXIF字段提取

### v0.1.0
- 初始版本
