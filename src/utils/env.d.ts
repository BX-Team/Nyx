/// <reference types="vite/client" />

declare module '*.worker?worker' {
  const WorkerFactory: {
    new (options?: WorkerOptions): Worker;
  };

  export default WorkerFactory;
}

declare module 'monaco-editor/esm/vs/editor/editor.worker?worker' {
  const WorkerFactory: {
    new (options?: WorkerOptions): Worker;
  };

  export default WorkerFactory;
}

declare module 'monaco-editor/esm/vs/language/css/css.worker?worker' {
  const WorkerFactory: {
    new (options?: WorkerOptions): Worker;
  };

  export default WorkerFactory;
}

declare module 'monaco-editor/esm/vs/language/html/html.worker?worker' {
  const WorkerFactory: {
    new (options?: WorkerOptions): Worker;
  };

  export default WorkerFactory;
}

declare module 'monaco-editor/esm/vs/language/json/json.worker?worker' {
  const WorkerFactory: {
    new (options?: WorkerOptions): Worker;
  };

  export default WorkerFactory;
}

declare module 'monaco-editor/esm/vs/language/typescript/ts.worker?worker' {
  const WorkerFactory: {
    new (options?: WorkerOptions): Worker;
  };

  export default WorkerFactory;
}

declare module 'monaco-yaml/yaml.worker?worker' {
  const WorkerFactory: {
    new (options?: WorkerOptions): Worker;
  };

  export default WorkerFactory;
}

declare module 'types-pac/pac.d.ts?raw' {
  const content: string;
  export default content;
}

declare module 'monaco-editor/esm/vs/base/common/errors.js' {
  export const errorHandler: {
    unexpectedErrorHandler: (e: Error) => void;
  };
}
