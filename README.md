# 📸 EXIF Catcher

一个用Rust编写的高性能EXIF信息提取API服务，可以从S3或任何公开URL的图片中提取拍摄参数。

## ✨ 功能特性

- 🚀 从S3或任何公开URL下载图片
- 📷 提取完整的EXIF信息（ISO、快门、光圈、相机型号等）
- 🔥 高性能异步处理
- 🐳 Docker容器化部署
- 🌐 CORS支持，方便前端调用

## 📋 提取的EXIF信息

- 相机制造商和型号
- 镜头型号
- ISO感光度
- 快门速度
- 光圈值
- 焦距
- 拍摄日期时间
- 图片分辨率

## 🚀 快速开始

### 本地开发

#### 1. 安装Rust（如果还没有）
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### 2. 运行项目
```bash
# 开发模式（有详细日志）
cargo run

# 生产模式（性能优化）
cargo build --release
./target/release/exif_catcher
```

服务将在 `http://localhost:3000` 启动

### 🐳 Docker部署

#### 构建镜像
```bash
docker build -t exif_catcher .
```

#### 运行容器
```bash
docker run -d -p 3000:3000 --name exif_catcher exif_catcher
```

#### 使用自定义端口
```bash
docker run -d -p 8080:8080 -e PORT=8080 --name exif_catcher exif_catcher
```

## 📖 API使用

### 健康检查
```bash
curl http://localhost:3000/health
```

响应：
```json
{
  "status": "ok",
  "service": "exif_catcher",
  "version": "0.1.0"
}
```

### 获取EXIF信息
```bash
curl "http://localhost:3000/exif?url=https://example.com/photo.jpg"
```

响应示例：
```json
{
  "success": true,
  "data": {
    "make": "Canon",
    "model": "Canon EOS R5",
    "lens": "RF24-70mm F2.8 L IS USM",
    "iso": "400",
    "shutter_speed": "1/1000",
    "aperture": "f/2.8",
    "focal_length": "50 mm",
    "date_time": "2025:12:17 14:30:00",
    "width": "8192",
    "height": "5464"
  }
}
```

### 前端调用示例

#### JavaScript/Fetch
```javascript
const imageUrl = 'https://your-s3-bucket.s3.amazonaws.com/photo.jpg';
const response = await fetch(`http://localhost:3000/exif?url=${encodeURIComponent(imageUrl)}`);
const data = await response.json();

if (data.success) {
  console.log('相机:', data.data.model);
  console.log('ISO:', data.data.iso);
  console.log('快门:', data.data.shutter_speed);
}
```

#### 使用axios
```javascript
import axios from 'axios';

const getExifData = async (imageUrl) => {
  const { data } = await axios.get('http://localhost:3000/exif', {
    params: { url: imageUrl }
  });
  return data;
};
```

## 🛠️ 开发说明

### 项目结构
```
exif_catcher/
├── Cargo.toml          # 依赖配置（类似Python的requirements.txt）
├── src/
│   └── main.rs         # 主程序代码
├── Dockerfile          # Docker配置
└── README.md           # 说明文档
```

### 关键依赖
- `axum` - Web框架
- `tokio` - 异步运行时
- `reqwest` - HTTP客户端
- `kamadak-exif` - EXIF解析
- `serde` - JSON序列化

### 调试技巧
```bash
# 查看详细日志
RUST_LOG=debug cargo run

# 运行测试
cargo test

# 代码格式化
cargo fmt

# 代码检查
cargo clippy
```

## 🌐 部署到生产环境

### 使用Docker Compose
创建 `docker-compose.yml`:
```yaml
version: '3.8'
services:
  exif_catcher:
    build: .
    ports:
      - "3000:3000"
    environment:
      - PORT=3000
    restart: unless-stopped
```

运行：
```bash
docker-compose up -d
```

### 环境变量
- `PORT` - 服务端口（默认：3000）
- `RUST_LOG` - 日志级别（debug/info/warn/error）

## 📝 注意事项

1. **图片URL必须公开可访问** - 确保S3 bucket的图片有公开读取权限
2. **图片格式** - 支持JPEG（JPG）格式，其他格式可能不含EXIF
3. **CORS** - 已启用CORS，可以从任何域名的前端调用
4. **性能** - 图片会下载到内存处理，大图片会占用较多内存

## 🤝 贡献

欢迎提交Issue和Pull Request！

## 📄 许可证

MIT License
