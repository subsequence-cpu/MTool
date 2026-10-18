.DEFAULT_GOAL := help

PNPM ?= pnpm
CARGO ?= cargo
TARGET ?=
BUNDLES ?=

TARGET_ARG = $(if $(strip $(TARGET)),--target "$(TARGET)")
BUNDLES_ARG = $(if $(strip $(BUNDLES)),--bundles "$(BUNDLES)")

ifeq ($(OS),Windows_NT)
HOST_PLATFORM := Windows
else
HOST_PLATFORM := $(shell uname -s)
endif

.PHONY: help install dev preview bindings fmt fmt-check typecheck clippy check \
	build package package-app package-dmg package-exe package-msi signing-setup \
	clean require-macos require-windows

help: ## 显示常用命令
	@node -e "const fs = require('node:fs'); console.log('MTool 常用命令\n'); for (const line of fs.readFileSync('Makefile', 'utf8').split(/\r?\n/)) { const match = line.match(/^([a-z][a-z0-9-]*):.*?## (.+)$$/); if (match) console.log('  make ' + match[1].padEnd(18) + match[2]); } console.log('\n打包参数：TARGET=<Rust target> BUNDLES=<app,dmg,nsis,msi>');"

install: ## 安装前端依赖（使用锁文件）
	$(PNPM) install --frozen-lockfile

dev: ## 启动 Tauri 桌面开发环境
	$(PNPM) desktop

preview: ## 启动浏览器样式预览
	$(PNPM) dev

bindings: ## 从 Rust 生成共享类型、默认设置与工具目录
	$(PNPM) bindings

fmt: ## 格式化 Rust、前端、配置与文档
	$(CARGO) fmt --all
	$(PNPM) format
ifeq ($(HOST_PLATFORM),Darwin)
	xcrun swift-format format --in-place native/finder-extension/FinderSync.swift
endif

fmt-check: ## 检查代码与文档格式
	$(CARGO) fmt --all --check
	$(PNPM) format:check
ifeq ($(HOST_PLATFORM),Darwin)
	xcrun swift-format lint --strict native/finder-extension/FinderSync.swift
endif

typecheck: ## 检查 Vue / TypeScript 类型
	$(PNPM) typecheck

clippy: ## 检查整个 Rust 工作区，警告视为错误
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings

check: fmt-check typecheck clippy ## 执行格式、类型与 Clippy 检查

build: ## 构建前端与 Rust 工作区（不生成安装包）
	$(PNPM) build
	$(CARGO) build --workspace --locked $(TARGET_ARG)

package: ## 为当前系统打包，复用现有签名流程
	$(PNPM) package $(BUNDLES_ARG) $(TARGET_ARG)

package-app: require-macos ## 在 macOS 上生成 .app
	$(PNPM) package --bundles app $(TARGET_ARG)

package-dmg: require-macos ## 在 macOS 上生成 .app 和 .dmg 安装包
	$(PNPM) package --bundles dmg $(TARGET_ARG)

package-exe: require-windows ## 在 Windows 上生成 NSIS .exe 安装包
	$(PNPM) package --bundles nsis $(TARGET_ARG)

package-msi: require-windows ## 在 Windows 上生成 .msi 安装包
	$(PNPM) package --bundles msi $(TARGET_ARG)

signing-setup: require-macos ## 手动配置一次 macOS 本地开发签名
	$(PNPM) signing:setup

clean: ## 清理构建产物，保留应用数据与依赖
	$(CARGO) clean
	node -e "const fs = require('node:fs'); for (const path of ['dist', 'native/gen/finder']) fs.rmSync(path, { recursive: true, force: true });"

require-macos:
ifneq ($(HOST_PLATFORM),Darwin)
	$(error 此命令需要在 macOS 上执行)
endif

require-windows:
ifneq ($(HOST_PLATFORM),Windows)
	$(error 此命令需要在 Windows 上执行，请使用 Windows 开发环境或 CI)
endif
