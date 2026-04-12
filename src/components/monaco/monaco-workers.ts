import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import CssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker';
import HtmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker';
import JsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker';
import TsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker';
import YamlWorker from 'monaco-yaml/yaml.worker?worker';

type WorkerFactory = () => Worker;

const workerFactories: Record<string, WorkerFactory> = {
  editorWorkerService: () => new EditorWorker(),
  css: () => new CssWorker(),
  scss: () => new CssWorker(),
  less: () => new CssWorker(),
  html: () => new HtmlWorker(),
  handlebars: () => new HtmlWorker(),
  razor: () => new HtmlWorker(),
  json: () => new JsonWorker(),
  typescript: () => new TsWorker(),
  javascript: () => new TsWorker(),
  yaml: () => new YamlWorker(),
};

const moduleWorkerFactories: Array<{ token: string; create: WorkerFactory }> = [
  { token: 'monaco-yaml/yaml.worker', create: () => new YamlWorker() },
  { token: 'vs/language/typescript/ts.worker', create: () => new TsWorker() },
  { token: 'vs/language/css/css.worker', create: () => new CssWorker() },
  { token: 'vs/language/html/html.worker', create: () => new HtmlWorker() },
  { token: 'vs/language/json/json.worker', create: () => new JsonWorker() },
  { token: 'vs/editor/editor.worker', create: () => new EditorWorker() },
];

const resolveWorkerByModuleId = (moduleId: string): WorkerFactory | undefined => {
  return moduleWorkerFactories.find(item => moduleId.includes(item.token))?.create;
};

const globalObject = globalThis as typeof globalThis & {
  MonacoEnvironment?: {
    getWorker?: (moduleId: string, label: string) => Worker;
    [key: string]: unknown;
  };
};

const previousEnvironment = globalObject.MonacoEnvironment ?? {};

globalObject.MonacoEnvironment = {
  ...previousEnvironment,
  getWorker(moduleId: string, label: string): Worker {
    const createWorker =
      workerFactories[label] ?? resolveWorkerByModuleId(moduleId) ?? workerFactories.editorWorkerService;
    return createWorker();
  },
};
