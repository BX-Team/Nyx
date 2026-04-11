import { t } from 'i18next';
import type React from 'react';
import { Button } from '@/components/ui/button';
import { Dialog, DialogClose, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { InputGroup, InputGroupAddon, InputGroupInput, InputGroupText } from '@/components/ui/input-group';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { useAppConfig } from '@/hooks/use-app-config';
import { restartMihomoConnections } from '@/utils/ipc';
import SettingItem from '../base/base-setting-item';

interface Props {
  onClose: () => void;
}

const ConnectionSettingModal: React.FC<Props> = props => {
  const { onClose } = props;
  const { appConfig, patchAppConfig } = useAppConfig();

  const { displayIcon = true, connectionInterval = 500, connectionListMode = 'process' } = appConfig || {};

  return (
    <Dialog
      open={true}
      onOpenChange={open => {
        if (!open) onClose();
      }}
    >
      <DialogContent className='flag-emoji max-w-lg' showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>{t('pages.connections.connectionSettings')}</DialogTitle>
        </DialogHeader>
        <div className='flex flex-col gap-1 py-2'>
          <SettingItem title={t('pages.connections.connectionListMode')} divider>
            <Select
              value={connectionListMode}
              onValueChange={v => {
                patchAppConfig({ connectionListMode: v as 'classic' | 'process' });
              }}
            >
              <SelectTrigger className='w-45'>
                <SelectValue />
              </SelectTrigger>
              <SelectContent position='popper'>
                <SelectItem value='classic'>{t('pages.connections.classicView')}</SelectItem>
                <SelectItem value='process'>{t('pages.connections.processView')}</SelectItem>
              </SelectContent>
            </Select>
          </SettingItem>
          <SettingItem title={t('connection.showAppIcon')} divider>
            <Switch
              checked={displayIcon}
              onCheckedChange={v => {
                patchAppConfig({ displayIcon: v });
              }}
            />
          </SettingItem>
          <SettingItem title={t('connection.refreshInterval')}>
            <InputGroup className='w-37.5'>
              <InputGroupInput
                type='number'
                value={connectionInterval?.toString()}
                placeholder={t('connection.refreshIntervalPlaceholder')}
                onChange={async e => {
                  let num = parseInt(e.target.value, 10);
                  if (Number.isNaN(num)) num = 500;
                  if (num < 100) num = 100;
                  await patchAppConfig({ connectionInterval: num });
                  await restartMihomoConnections();
                }}
              />
              <InputGroupAddon align='inline-end'>
                <InputGroupText>{t('connection.refreshIntervalUnit')}</InputGroupText>
              </InputGroupAddon>
            </InputGroup>
          </SettingItem>
        </div>
        <DialogFooter>
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

export default ConnectionSettingModal;
