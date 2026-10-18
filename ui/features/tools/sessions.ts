import { reactive } from 'vue';
import type { ToolDefinition, ToolId } from '../../bindings/types';

interface ToolSession {
  input: string;
  secondary: string;
  operation: string;
  options: Record<string, string>;
  output: string;
  notice: string | null;
  error: string;
  working: boolean;
  sequence: number;
  elapsedMs: number | null;
}

const sessions = reactive(new Map<ToolId, ToolSession>());

export function getSession(tool: ToolDefinition): ToolSession {
  let session = sessions.get(tool.id);
  if (!session) {
    session = reactive({
      input: tool.sample,
      secondary: tool.id === 'diff' ? 'name: MTool\nversion: 0.2.0\noffline: true\n' : '',
      operation: tool.operations[0]?.id ?? '',
      options: {
        algorithm: 'sha256',
        key: '',
        unit: 'seconds',
        count: '1',
        length: '32',
        pattern: '[\\w.+-]+@[\\w.-]+\\.[A-Za-z]{2,}',
        replacement: '',
      },
      output: '',
      notice: null,
      error: '',
      working: false,
      sequence: 0,
      elapsedMs: null,
    });
    sessions.set(tool.id, session);
  }
  return session;
}
