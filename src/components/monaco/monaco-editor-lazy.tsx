import React, { Suspense } from 'react';
import { Spinner } from '@/components/ui/spinner';

const BaseEditorComponent = React.lazy(() =>
  import('./monaco-editor').then(module => ({ default: module.BaseEditor })),
);

type Language = 'yaml' | 'javascript' | 'css' | 'json' | 'text';

interface Props {
  value: string;
  originalValue?: string;
  diffRenderSideBySide?: boolean;
  readOnly?: boolean;
  language: Language;
  onChange?: (value: string) => void;
}

export const BaseEditor: React.FC<Props> = props => {
  return (
    <Suspense
      fallback={
        <div className='w-full h-full flex items-center justify-center'>
          <Spinner className='size-8' />
        </div>
      }
    >
      <BaseEditorComponent {...props} />
    </Suspense>
  );
};
