<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import { appState, notify, toggleFavorite } from '../../app/store';
import { client, desktopAvailable, errorMessage } from '../../bindings/client';
import AppIcon from '../../components/ui/AppIcon.vue';
import AppSelect from '../../components/ui/AppSelect.vue';
import { getSession } from './sessions';

const CodeEditor = defineAsyncComponent(() => import('../../components/editor/CodeEditor.vue'));
const route = useRoute();
const tool = computed(() => appState.tools.find((item) => item.id === route.params.toolId));
const session = computed(() => (tool.value ? getSession(tool.value) : null));
const showKey = ref(false);
const modifier = navigator.userAgent.includes('Mac') ? '⌘' : 'Ctrl';
const inputSize = computed(() => new TextEncoder().encode(session.value?.input ?? '').length);
const secondarySize = computed(
  () => new TextEncoder().encode(session.value?.secondary ?? '').length,
);

async function run(): Promise<void> {
  const current = session.value;
  const selected = tool.value;
  if (!current || !selected || current.working || !desktopAvailable) return;
  current.working = true;
  current.error = '';
  const sequence = ++current.sequence;
  const started = performance.now();
  try {
    const result = await client.runTool({
      tool: selected.id,
      operation: current.operation,
      input: current.input,
      secondary: current.secondary,
      options: { ...current.options },
    });
    if (sequence !== current.sequence) return;
    current.output = result.output;
    current.notice = result.notice;
    current.elapsedMs = Math.round(performance.now() - started);
  } catch (error) {
    if (sequence === current.sequence) {
      current.error = errorMessage(error);
      current.notice = null;
    }
  } finally {
    if (sequence === current.sequence) current.working = false;
  }
}

function cancel(): void {
  if (!session.value) return;
  session.value.sequence += 1;
  session.value.working = false;
  session.value.notice = '已取消等待';
}

function clear(): void {
  if (!session.value || session.value.working) return;
  Object.assign(session.value, {
    input: '',
    secondary: '',
    output: '',
    error: '',
    notice: null,
    elapsedMs: null,
  });
}

function loadSample(): void {
  const current = session.value;
  if (!tool.value || !current || current.working) return;
  current.input =
    tool.value.id === 'timestamp' && current.operation === 'to-timestamp'
      ? '2026-10-16T09:00:00+08:00'
      : tool.value.sample;
  if (tool.value.id === 'diff') current.secondary = 'name: MTool\nversion: 0.2.0\noffline: true\n';
}

async function copyResult(): Promise<void> {
  const current = session.value;
  if (!current) return;
  try {
    await client.copyText(current.output);
    notify('已复制');
  } catch (error) {
    notify(errorMessage(error), true);
  }
}

function keyboard(event: KeyboardEvent): void {
  if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
    event.preventDefault();
    void run();
  }
}
onMounted(() => window.addEventListener('keydown', keyboard));
onUnmounted(() => window.removeEventListener('keydown', keyboard));
</script>

<template>
  <section v-if="tool && session" class="workspace-page">
    <RouterLink to="/" class="back-link"><AppIcon name="arrow-left" :size="14" />工具箱</RouterLink>
    <header class="page-heading">
      <div>
        <h1>{{ tool.title }}</h1>
        <p class="subtitle">{{ tool.description }}</p>
      </div>
      <button
        class="icon-button"
        :class="{ 'is-favorite': appState.settings.favorites.includes(tool.id) }"
        :aria-label="appState.settings.favorites.includes(tool.id) ? '取消收藏' : '收藏此工具'"
        @click="toggleFavorite(tool.id)"
      >
        <AppIcon name="star" />
      </button>
    </header>
    <div class="workspace-toolbar">
      <label class="inline-field"
        ><span class="sr-only">操作</span
        ><AppSelect v-model="session.operation" :icon="tool.icon" :disabled="session.working">
          <option v-for="operation in tool.operations" :key="operation.id" :value="operation.id">
            {{ operation.label }}
          </option>
        </AppSelect></label
      ><button class="button primary" :disabled="session.working || !desktopAvailable" @click="run">
        <AppIcon :name="session.working ? 'clock' : 'play'" :size="15" />{{
          session.working ? '处理中…' : '运行'
        }}<kbd>{{ modifier }} ↵</kbd></button
      ><button v-if="session.working" class="button" @click="cancel">
        <AppIcon name="x" />取消等待</button
      ><span class="toolbar-spacer" /><button
        class="button"
        :disabled="session.working"
        @click="loadSample"
      >
        <AppIcon name="file-text" />载入示例</button
      ><button class="button" :disabled="session.working" @click="clear">
        <AppIcon name="eraser" />清空
      </button>
    </div>
    <div v-if="tool.id === 'hash'" class="tool-options">
      <label class="field"
        ><span>算法</span
        ><AppSelect v-model="session.options.algorithm" icon="fingerprint">
          <option value="sha256">SHA-256</option>
          <option value="sha512">SHA-512</option>
          <option value="sha1">SHA-1</option>
          <option value="md5">MD5</option>
        </AppSelect></label
      ><label v-if="session.operation === 'hmac'" class="field grow"
        ><span>HMAC 密钥</span
        ><span class="input-action"
          ><input
            v-model="session.options.key"
            :type="showKey ? 'text' : 'password'"
            autocomplete="off"
          /><button class="button ghost" @click="showKey = !showKey">
            <AppIcon :name="showKey ? 'eye-off' : 'eye'" />
            {{ showKey ? '隐藏' : '显示' }}
          </button></span
        ></label
      >
    </div>
    <div
      v-if="tool.id === 'timestamp' && session.operation === 'from-timestamp'"
      class="tool-options"
    >
      <label class="field"
        ><span>输入单位</span
        ><AppSelect v-model="session.options.unit" icon="clock">
          <option value="seconds">秒</option>
          <option value="milliseconds">毫秒</option>
        </AppSelect></label
      >
      <p class="muted">输出同时展示 UTC、系统本地时区及两种时间戳。</p>
    </div>
    <div v-if="tool.id === 'regex'" class="tool-options">
      <label class="field grow"
        ><span>正则表达式</span
        ><input
          v-model="session.options.pattern"
          class="mono"
          spellcheck="false"
          placeholder="例如：\d+"
          maxlength="4096" /></label
      ><label v-if="session.operation === 'replace'" class="field grow"
        ><span>替换内容（支持 $1、${name}）</span
        ><input v-model="session.options.replacement" spellcheck="false"
      /></label>
      <p class="option-note">Rust regex 语法，不支持回溯引用和前后查找；位置以 UTF-8 字节计。</p>
    </div>
    <div v-if="tool.id === 'uuid'" class="tool-options">
      <label class="field"
        ><span>生成数量</span
        ><input v-model="session.options.count" type="number" min="1" max="100" /></label
      ><label v-if="session.operation === 'random'" class="field"
        ><span>每条长度</span
        ><input v-model="session.options.length" type="number" min="1" max="1024"
      /></label>
    </div>
    <p v-if="tool.id === 'timestamp' && session.operation === 'to-timestamp'" class="inline-notice">
      输入须带时区，例如 2026-10-16T09:00:00+08:00。
    </p>
    <div class="editors" :class="{ 'single-editor': tool.id === 'uuid' }">
      <section v-if="tool.id !== 'uuid'" class="editor-panel">
        <header>
          <span>{{ tool.id === 'diff' ? '原始文本' : '输入' }}</span
          ><span class="muted">UTF-8</span>
        </header>
        <CodeEditor
          v-model="session.input"
          :json="tool.id === 'json' || tool.id === 'jwt'"
          label="工具输入"
        />
        <footer>
          <span>{{ inputSize.toLocaleString() }} 字节</span>
        </footer>
      </section>
      <section v-if="tool.id === 'diff'" class="editor-panel">
        <header><span>修改后文本</span><span class="muted">UTF-8</span></header>
        <CodeEditor v-model="session.secondary" label="修改后文本" />
        <footer>{{ secondarySize.toLocaleString() }} 字节</footer>
      </section>
      <section v-else class="editor-panel">
        <header>
          <span>输出</span
          ><button
            class="button ghost small"
            :disabled="!session.output || !desktopAvailable"
            @click="copyResult()"
          >
            <AppIcon name="copy" :size="14" />复制
          </button>
        </header>
        <CodeEditor
          :model-value="session.output"
          :json="['json', 'jwt', 'regex'].includes(tool.id)"
          label="工具输出"
          readonly
        />
        <footer>
          <span>{{ session.elapsedMs !== null ? `${session.elapsedMs} ms` : '待运行' }}</span>
        </footer>
      </section>
    </div>
    <section v-if="tool.id === 'diff' && session.output" class="editor-panel diff-output">
      <header>
        <span>差异结果</span
        ><button class="button ghost small" @click="copyResult()">
          <AppIcon name="copy" :size="14" />复制
        </button>
      </header>
      <pre
        class="diff-content"
      ><span v-for="(line, index) in session.output.split('\n')" :key="index" :class="{ 'diff-added': line.startsWith('+'), 'diff-removed': line.startsWith('-') }">{{ line }}{{ '\n' }}</span></pre>
    </section>
    <p v-if="session.error" class="inline-notice error" role="alert">
      <AppIcon name="alert" :size="16" />{{ session.error }}
    </p>
    <p v-if="session.notice" class="inline-notice" role="status">
      <AppIcon name="info" :size="16" />{{ session.notice }}
    </p>
    <p
      v-if="!session.error && !session.notice && session.elapsedMs !== null"
      class="success-message"
      role="status"
    >
      <AppIcon name="check" :size="15" />已完成
    </p>
  </section>
  <section v-else class="empty-state">
    <h1>没有找到这个工具</h1>
    <RouterLink to="/" class="button"><AppIcon name="arrow-left" />返回工具箱</RouterLink>
  </section>
</template>
