import { t } from 'i18next';
import type React from 'react';
import { useCallback, useEffect, useState } from 'react';
import useSWR from 'swr';
import { Button } from '@/components/ui/button';
import { Dialog, DialogClose, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Switch } from '@/components/ui/switch';
import { getCurrentProfileStr, getProfileConfig, getRawProfileStr, getRuntimeConfigStr } from '@/utils/ipc';
import { BaseEditor } from '../monaco/monaco-editor-lazy';

interface Props {
  onClose: () => void;
}
const ConfigViewer: React.FC<Props> = ({ onClose }) => {
  const [runtimeConfig, setRuntimeConfig] = useState('');
  const [rawProfile, setRawProfile] = useState('');
  const [profileConfig, setProfileConfig] = useState('');
  const [isDiff, setIsDiff] = useState(false);
  const [isRaw, setIsRaw] = useState(false);
  const [sideBySide, setSideBySide] = useState(false);

  const { data: config } = useSWR('getProfileConfig', getProfileConfig);

  const fetchConfigs = useCallback(async () => {
    setRuntimeConfig(await getRuntimeConfigStr());
    setRawProfile(await getRawProfileStr());
    setProfileConfig(await getCurrentProfileStr());
  }, []);

  useEffect(() => {
    fetchConfigs();
  }, [fetchConfigs]);

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
        <DialogHeader className='app-drag'>
          <DialogTitle>{t('sider.runtimeConfigTitle')}</DialogTitle>
        </DialogHeader>
        <div className='flex-1 min-h-0'>
          <BaseEditor
            language='yaml'
            value={runtimeConfig}
            originalValue={isDiff ? (isRaw ? rawProfile : profileConfig) : undefined}
            readOnly
            diffRenderSideBySide={sideBySide}
          />
        </div>
        <DialogFooter className='flex items-center justify-between sm:justify-between'>
          <div className='flex items-center space-x-4'>
            <label className='flex items-center space-x-2 text-sm'>
              <Switch checked={isDiff} onCheckedChange={setIsDiff} />
              <span>{t('sider.compareCurrentConfig')}</span>
            </label>
            <label className='flex items-center space-x-2 text-sm'>
              <Switch checked={sideBySide} onCheckedChange={setSideBySide} />
              <span>{t('sider.sideBySide')}</span>
            </label>
            <label className='flex items-center space-x-2 text-sm'>
              <Switch checked={isRaw} onCheckedChange={setIsRaw} />
              <span>{t('sider.showRawText')}</span>
            </label>
          </div>
          <DialogClose asChild>
            <Button size='sm' variant='ghost'>
              {t('common.close')}
            </Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default ConfigViewer;
