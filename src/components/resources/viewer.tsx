import { t } from 'i18next';
import yaml from 'js-yaml';
import type React from 'react';
import { useEffect, useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Dialog, DialogClose, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { convertMrsRuleset, getFileStr, getRuntimeConfig, setFileStr } from '@/utils/ipc';
import { BaseEditor } from '../monaco/monaco-editor-lazy';

type Language = 'yaml' | 'javascript' | 'css' | 'json' | 'text';

interface Props {
  onClose: () => void;
  path: string;
  type: string;
  title: string;
  privderType: string;
  format?: string;
  behavior?: string;
}
const Viewer: React.FC<Props> = props => {
  const { type, path, title, format, privderType, behavior, onClose } = props;
  const [currData, setCurrData] = useState('');
  const [isLoading, setIsLoading] = useState(true);

  const language: Language = useMemo(() => {
    if (format === 'MrsRule') return 'text';
    if (type === 'Inline') return 'yaml';
    if (!format || format === 'YamlRule') return 'yaml';
    return 'text';
  }, [format, type]);

  useEffect(() => {
    const loadContent = async (): Promise<void> => {
      setIsLoading(true);
      try {
        let fileContent: React.SetStateAction<string>;

        if (format === 'MrsRule') {
          let ruleBehavior: string = behavior || 'domain';
          if (!behavior) {
            try {
              const runtimeConfig = await getRuntimeConfig();
              const provider = runtimeConfig['rule-providers']?.providers?.[title];
              ruleBehavior = provider?.behavior || 'domain';
            } catch {
              ruleBehavior = 'domain';
            }
          }

          fileContent = await convertMrsRuleset(path, ruleBehavior);
          setCurrData(fileContent);
          return;
        }

        if (type === 'Inline') {
          fileContent = await getFileStr('config.yaml');
        } else {
          fileContent = await getFileStr(path);
        }
        try {
          const parsedYaml = yaml.load(fileContent);
          if (parsedYaml && typeof parsedYaml === 'object') {
            const yamlObj = parsedYaml as Record<string, unknown>;
            const providerObj = yamlObj[privderType];
            const providerMap =
              providerObj && typeof providerObj === 'object'
                ? (providerObj as Record<string, Record<string, unknown>>)
                : undefined;
            const targetObj = providerMap?.[title];
            const payload = targetObj?.payload;
            if (payload) {
              if (privderType === 'proxy-providers') {
                setCurrData(
                  yaml.dump({
                    proxies: payload,
                  }),
                );
              } else {
                setCurrData(
                  yaml.dump({
                    rules: payload,
                  }),
                );
              }
            } else {
              if (targetObj) {
                setCurrData(yaml.dump(targetObj));
              } else {
                setCurrData(fileContent);
              }
            }
          } else {
            setCurrData(fileContent);
          }
        } catch (_error) {
          setCurrData(fileContent);
        }
      } finally {
        setIsLoading(false);
      }
    };
    loadContent();
  }, [path, type, title, format, privderType, behavior]);

  return (
    <Dialog
      open={true}
      onOpenChange={open => {
        if (!open) onClose();
      }}
    >
      <DialogContent
        className='h-[calc(100%-111px)] w-[calc(100%-100px)] max-w-none sm:max-w-none flex flex-col'
        showCloseButton={false}
      >
        <DialogHeader className='app-drag pb-0'>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        <div className='flex-1 min-h-0'>
          {isLoading ? (
            <div className='flex items-center justify-center h-full'>
              <div className='text-foreground-500'>{t('common.loading')}</div>
            </div>
          ) : (
            <BaseEditor
              language={language}
              value={currData}
              readOnly={type !== 'File' || format === 'MrsRule'}
              onChange={value => setCurrData(value)}
            />
          )}
        </div>
        <DialogFooter className='pt-0'>
          <DialogClose asChild>
            <Button size='sm' variant='ghost'>
              {t('common.close')}
            </Button>
          </DialogClose>
          {type === 'File' && format !== 'MrsRule' && (
            <Button
              size='sm'
              onClick={async () => {
                await setFileStr(path, currData);
                onClose();
              }}
            >
              {t('common.save')}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default Viewer;
