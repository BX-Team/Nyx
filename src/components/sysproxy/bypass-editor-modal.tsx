import yaml from 'js-yaml';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { BaseEditor } from '../monaco/monaco-editor-lazy';

interface Props {
  bypass: string[];
  onCancel: () => void;
  onConfirm: (bypass: string[]) => void;
}

interface ParsedYaml {
  bypass?: string[];
}

const ByPassEditorModal: React.FC<Props> = props => {
  const { t } = useTranslation();
  const { bypass, onCancel, onConfirm } = props;
  const [currData, setCurrData] = useState<string>('');
  useEffect(() => {
    setCurrData(yaml.dump({ bypass }));
  }, [bypass]);
  const handleConfirm = (): void => {
    try {
      const parsed = yaml.load(currData) as ParsedYaml;
      if (parsed && Array.isArray(parsed.bypass)) {
        onConfirm(parsed.bypass);
      } else {
        toast.error(t('sysproxy.yamlFormatError'));
      }
    } catch (e) {
      toast.error(t('sysproxy.yamlParseFailed') + e);
    }
  };

  return (
    <Dialog
      open={true}
      onOpenChange={open => {
        if (!open) onCancel();
      }}
    >
      <DialogContent
        className='h-[calc(100%-111px)] w-[calc(100%-100px)] max-w-none sm:max-w-none flex flex-col'
        showCloseButton={false}
      >
        <DialogHeader className='app-drag pb-0'>
          <DialogTitle>{t('sysproxy.bypassEditorTitle')}</DialogTitle>
        </DialogHeader>
        <div className='flex-1 min-h-0'>
          <BaseEditor language='yaml' value={currData} onChange={value => setCurrData(value || '')} />
        </div>
        <DialogFooter className='pt-0'>
          <Button size='sm' variant='ghost' onClick={onCancel}>
            {t('common.cancel')}
          </Button>
          <Button size='sm' onClick={handleConfirm}>
            {t('common.confirm')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default ByPassEditorModal;
