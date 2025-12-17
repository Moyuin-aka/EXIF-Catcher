# 使用Rust官方镜像作为构建环境
FROM rust:1.75 as builder

# 设置工作目录
WORKDIR /app

# 复制依赖文件
COPY Cargo.toml Cargo.lock* ./

# 创建一个虚拟的main.rs来预编译依赖（加速后续构建）
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# 复制源代码
COPY src ./src

# 构建应用（依赖已缓存，只编译我们的代码）
RUN cargo build --release

# ============== 运行时镜像 ==============
# 使用更小的基础镜像来运行
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 从构建阶段复制编译好的二进制文件
COPY --from=builder /app/target/release/exif_catcher /app/exif_catcher

# 暴露端口
EXPOSE 3000

# 设置环境变量
ENV PORT=3000

# 运行应用
CMD ["/app/exif_catcher"]
