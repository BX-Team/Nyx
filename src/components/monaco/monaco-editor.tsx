import { t } from 'i18next';
import metaSchema from 'meta-json-schema/schemas/meta-json-schema.json';
import * as monaco from 'monaco-editor';
import { errorHandler } from 'monaco-editor/esm/vs/base/common/errors.js';
import { configureMonacoYaml } from 'monaco-yaml';
import { nanoid } from 'nanoid';
import { useTheme } from 'next-themes';
import type React from 'react';
import { useRef } from 'react';
import MonacoEditor, { MonacoDiffEditor } from 'react-monaco-editor';
import pac from 'types-pac/pac.d.ts?raw';

type Language = 'yaml' | 'javascript' | 'css' | 'json' | 'text';

interface Props {
  value: string;
  originalValue?: string;
  diffRenderSideBySide?: boolean;
  readOnly?: boolean;
  language: Language;
  onChange?: (value: string) => void;
}

let initialized = false;
const monacoInitialization = (): void => {
  if (initialized) return;

  const originalHandler = errorHandler.unexpectedErrorHandler as (e: Error) => void;
  errorHandler.unexpectedErrorHandler = (e: Error): void => {
    if (e?.message?.startsWith('Missing requestHandler or method:')) return;
    originalHandler.call(errorHandler, e);
  };

  const prevWindowOnerror = window.onerror;
  window.onerror = (msg, _src, _line, _col, error): boolean => {
    const message = error?.message ?? (typeof msg === 'string' ? msg : String(msg));
    if (message.startsWith('Missing requestHandler or method:')) return true;
    if (prevWindowOnerror) return Boolean(prevWindowOnerror.call(window, msg, _src, _line, _col, error));
    return false;
  };

  const insertPrefixDescription = t('editor.schema.insertPrefix');
  const appendSuffixDescription = t('editor.schema.appendSuffix');
  const forceOverrideDescription = t('editor.schema.forceOverride');

  configureMonacoYaml(monaco, {
    validate: true,
    enableSchemaRequest: true,
    schemas: [
      {
        uri: 'http://example.com/meta-json-schema.json',
        fileMatch: ['**/*.clash.yaml'],
        schema: {
          ...metaSchema,
          patternProperties: {
            '\\+rules': {
              type: 'array',
              $ref: '#/definitions/rules',
              description: insertPrefixDescription,
            },
            'rules\\+': {
              type: 'array',
              $ref: '#/definitions/rules',
              description: appendSuffixDescription,
            },
            '\\+proxies': {
              type: 'array',
              $ref: '#/definitions/proxies',
              description: insertPrefixDescription,
            },
            'proxies\\+': {
              type: 'array',
              $ref: '#/definitions/proxies',
              description: appendSuffixDescription,
            },
            '\\+proxy-groups': {
              type: 'array',
              $ref: '#/definitions/proxy-groups',
              description: insertPrefixDescription,
            },
            'proxy-groups\\+': {
              type: 'array',
              $ref: '#/definitions/proxy-groups',
              description: appendSuffixDescription,
            },
            '^\\+': {
              type: 'array',
              description: insertPrefixDescription,
            },
            '\\+$': {
              type: 'array',
              description: appendSuffixDescription,
            },
            '!$': {
              type: 'object',
              description: forceOverrideDescription,
            },
          },
        },
      },
    ],
  });
  monaco.languages.typescript.javascriptDefaults.addExtraLib(pac, 'pac.d.ts');
  initialized = true;
};

export const BaseEditor: React.FC<Props> = props => {
  const { theme, systemTheme } = useTheme();
  const trueTheme = theme === 'system' ? systemTheme : theme;
  const { value, originalValue, diffRenderSideBySide = false, readOnly = false, language, onChange } = props;

  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor>(undefined);
  const diffEditorRef = useRef<monaco.editor.IStandaloneDiffEditor>(undefined);

  const editorWillMount = (): void => {
    monacoInitialization();
  };

  const editorDidMount = (editor: monaco.editor.IStandaloneCodeEditor): void => {
    editorRef.current = editor;

    const prevModel = editor.getModel();
    const uri = monaco.Uri.parse(`${nanoid()}.${language === 'yaml' ? 'clash' : ''}.${language}`);
    const model = monaco.editor.createModel(value, language, uri);
    editorRef.current.setModel(model);
    prevModel?.dispose();
  };
  const diffEditorDidMount = (editor: monaco.editor.IStandaloneDiffEditor): void => {
    diffEditorRef.current = editor;

    const originalUri = monaco.Uri.parse(`original-${nanoid()}.${language === 'yaml' ? 'clash' : ''}.${language}`);
    const modifiedUri = monaco.Uri.parse(`modified-${nanoid()}.${language === 'yaml' ? 'clash' : ''}.${language}`);
    const originalModel = monaco.editor.createModel(originalValue || '', language, originalUri);
    const modifiedModel = monaco.editor.createModel(value, language, modifiedUri);
    diffEditorRef.current.setModel({
      original: originalModel,
      modified: modifiedModel,
    });
  };

  const options = {
    tabSize: ['yaml', 'javascript', 'json'].includes(language) ? 2 : 4,
    minimap: {
      enabled: document.documentElement.clientWidth >= 1500,
    },
    mouseWheelZoom: true,
    readOnly: readOnly,
    renderValidationDecorations: 'on' as 'off' | 'on' | 'editable',
    quickSuggestions: {
      strings: true,
      comments: true,
      other: true,
    },
    fontFamily: `Maple Mono NF CN,Fira Code, JetBrains Mono, Roboto Mono, "Source Code Pro", Consolas, Menlo, Monaco, monospace, "Courier New", "Apple Color Emoji", "Noto Color Emoji"`,
    fontLigatures: true,
    smoothScrolling: true,
    pixelRatio: window.devicePixelRatio,
    renderSideBySide: diffRenderSideBySide,
    glyphMargin: false,
    folding: true,
    scrollBeyondLastLine: false,
    automaticLayout: true,
    wordWrap: 'on' as const,
    cursorBlinking: 'blink' as const,
    cursorSmoothCaretAnimation: 'off' as const,
    scrollbar: {
      useShadows: true,
      verticalScrollbarSize: 14,
      horizontalScrollbarSize: 14,
    },
    suggest: {
      insertMode: 'insert' as const,
      showIcons: true,
    },
    hover: {
      enabled: true,
      delay: 300,
    },
  };

  if (originalValue !== undefined) {
    return (
      <MonacoDiffEditor
        language={language}
        original={originalValue}
        value={value}
        height='100%'
        theme={trueTheme?.includes('light') ? 'vs' : 'vs-dark'}
        options={options}
        editorWillMount={editorWillMount}
        editorDidMount={diffEditorDidMount}
        editorWillUnmount={(editor): void => {
          const models = editor.getModel();
          models?.original.dispose();
          models?.modified.dispose();
        }}
        onChange={onChange}
      />
    );
  }

  return (
    <MonacoEditor
      language={language}
      value={value}
      height='100%'
      theme={trueTheme?.includes('light') ? 'vs' : 'vs-dark'}
      options={options}
      editorWillMount={editorWillMount}
      editorDidMount={editorDidMount}
      editorWillUnmount={(editor): void => {
        editor.getModel()?.dispose();
      }}
      onChange={onChange}
    />
  );
};
