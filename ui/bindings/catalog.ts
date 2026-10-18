import type { ToolDefinition } from './types';

export const catalog: ToolDefinition[] = [
  {
    "id": "json",
    "title": "JSON 工具",
    "description": "格式化 · 校验 · 压缩 · 转义",
    "category": "格式与文本",
    "icon": "braces",
    "keywords": [
      "json",
      "格式化",
      "校验",
      "转义"
    ],
    "operations": [
      {
        "id": "format",
        "label": "格式化"
      },
      {
        "id": "compact",
        "label": "压缩"
      },
      {
        "id": "validate",
        "label": "校验"
      },
      {
        "id": "escape",
        "label": "转义"
      },
      {
        "id": "unescape",
        "label": "去转义"
      }
    ],
    "sample": "{\"name\":\"MTool\",\"platforms\":[\"macOS\",\"Windows\"],\"offline\":true}"
  },
  {
    "id": "base64",
    "title": "Base64",
    "description": "编码 · 解码",
    "category": "编解码",
    "icon": "binary",
    "keywords": [
      "base64",
      "编码",
      "解码"
    ],
    "operations": [
      {
        "id": "encode",
        "label": "编码"
      },
      {
        "id": "decode",
        "label": "解码"
      }
    ],
    "sample": "Hello, MTool!"
  },
  {
    "id": "url",
    "title": "URL 编解码",
    "description": "编码 · 解码",
    "category": "编解码",
    "icon": "link",
    "keywords": [
      "url",
      "percent",
      "百分号"
    ],
    "operations": [
      {
        "id": "encode",
        "label": "编码"
      },
      {
        "id": "decode",
        "label": "解码"
      }
    ],
    "sample": "https://example.com/search?q=你好 世界"
  },
  {
    "id": "unicode",
    "title": "Unicode 转换",
    "description": "转义 · 还原",
    "category": "编解码",
    "icon": "languages",
    "keywords": [
      "unicode",
      "中文",
      "转义"
    ],
    "operations": [
      {
        "id": "encode",
        "label": "转义"
      },
      {
        "id": "decode",
        "label": "还原"
      }
    ],
    "sample": "你好，MTool 👋"
  },
  {
    "id": "hex",
    "title": "Hex / String",
    "description": "文本 ↔ Hex",
    "category": "编解码",
    "icon": "code",
    "keywords": [
      "hex",
      "十六进制",
      "string"
    ],
    "operations": [
      {
        "id": "encode",
        "label": "文本转 Hex"
      },
      {
        "id": "decode",
        "label": "Hex 转文本"
      }
    ],
    "sample": "Hello, 世界"
  },
  {
    "id": "timestamp",
    "title": "时间戳转换",
    "description": "时间戳 ↔ 日期",
    "category": "生成与转换",
    "icon": "clock",
    "keywords": [
      "timestamp",
      "时间",
      "日期",
      "毫秒"
    ],
    "operations": [
      {
        "id": "from-timestamp",
        "label": "时间戳 → 日期"
      },
      {
        "id": "to-timestamp",
        "label": "日期 → 时间戳"
      }
    ],
    "sample": "1792112400"
  },
  {
    "id": "hash",
    "title": "哈希计算",
    "description": "MD5 · SHA · HMAC",
    "category": "加密与校验",
    "icon": "fingerprint",
    "keywords": [
      "hash",
      "sha",
      "md5",
      "hmac"
    ],
    "operations": [
      {
        "id": "digest",
        "label": "哈希"
      },
      {
        "id": "hmac",
        "label": "HMAC"
      }
    ],
    "sample": "Hello, MTool!"
  },
  {
    "id": "jwt",
    "title": "JWT 解码",
    "description": "Header · Payload（未验签）",
    "category": "加密与校验",
    "icon": "key",
    "keywords": [
      "jwt",
      "token",
      "令牌"
    ],
    "operations": [
      {
        "id": "decode",
        "label": "解码"
      }
    ],
    "sample": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJtdG9vbCIsIm5hbWUiOiJN VG9vbCJ9.signature"
  },
  {
    "id": "uuid",
    "title": "UUID / 随机字符",
    "description": "UUID v4 · 随机字符串",
    "category": "生成与转换",
    "icon": "shuffle",
    "keywords": [
      "uuid",
      "random",
      "随机"
    ],
    "operations": [
      {
        "id": "uuid",
        "label": "UUID v4"
      },
      {
        "id": "random",
        "label": "随机字符串"
      }
    ],
    "sample": ""
  },
  {
    "id": "text",
    "title": "文本处理",
    "description": "去重 · 排序 · 大小写 · 统计",
    "category": "格式与文本",
    "icon": "text",
    "keywords": [
      "text",
      "文本",
      "去重",
      "统计"
    ],
    "operations": [
      {
        "id": "dedupe",
        "label": "按行去重"
      },
      {
        "id": "sort",
        "label": "按行排序"
      },
      {
        "id": "upper",
        "label": "大写"
      },
      {
        "id": "lower",
        "label": "小写"
      },
      {
        "id": "trim",
        "label": "清理空白"
      },
      {
        "id": "stats",
        "label": "统计"
      }
    ],
    "sample": "Rust\nTypeScript\nRust\nVue\n"
  },
  {
    "id": "diff",
    "title": "文本对比",
    "description": "逐行对比",
    "category": "格式与文本",
    "icon": "diff",
    "keywords": [
      "diff",
      "对比",
      "差异"
    ],
    "operations": [
      {
        "id": "compare",
        "label": "对比"
      }
    ],
    "sample": "name: MTool\nversion: 0.1.0\noffline: true\n"
  },
  {
    "id": "regex",
    "title": "正则表达式",
    "description": "匹配 · 捕获 · 替换",
    "category": "加密与校验",
    "icon": "regex",
    "keywords": [
      "regex",
      "正则",
      "匹配",
      "替换"
    ],
    "operations": [
      {
        "id": "match",
        "label": "匹配"
      },
      {
        "id": "replace",
        "label": "替换"
      }
    ],
    "sample": "contact: hello@example.com\nwebsite: example.com"
  }
];
