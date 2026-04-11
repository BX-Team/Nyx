import { t } from 'i18next';
import type React from 'react';
import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Dialog, DialogClose, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useAppConfig } from '@/hooks/use-app-config';
import debounce from '@/utils/debounce';
import SettingItem from '../base/base-setting-item';

interface Props {
  onClose: () => void;
}

const ProxySettingModal: React.FC<Props> = props => {
  const { onClose } = props;
  const { appConfig, patchAppConfig } = useAppConfig();

  const {
    proxyCols = 'auto',
    proxyDisplayOrder = 'default',
    groupDisplayLayout = 'single',
    proxyDisplayLayout = 'double',
    autoCloseConnection = true,
    expandProxyGroups = false,
    delayTestUrl,
    delayTestConcurrency,
    delayTestTimeout,
  } = appConfig || {};

  const [url, setUrl] = useState(delayTestUrl ?? '');

  const setUrlDebounce = debounce((v: string) => {
    patchAppConfig({ delayTestUrl: v });
  }, 500);

  useEffect(() => {
    setUrl(delayTestUrl ?? '');
  }, [delayTestUrl]);

  return (
    <Dialog
      open={true}
      onOpenChange={open => {
        if (!open) onClose();
      }}
    >
      <DialogContent
        className='flag-emoji sm:max-w-xl max-h-[calc(100vh-120px)] flex flex-col min-h-0'
        showCloseButton={false}
      >
        <DialogHeader className='pb-0'>
          <DialogTitle>{t('pages.proxies.proxyGroupSettings')}</DialogTitle>
        </DialogHeader>
        <div className='py-2 flex flex-col gap-1 overflow-y-auto min-h-0'>
          <SettingItem title={t('proxies.proxyNodeColumns')} divider>
            <Select
              value={proxyCols}
              onValueChange={async value => {
                await patchAppConfig({ proxyCols: value as 'auto' | '1' | '2' | '3' | '4' });
              }}
            >
              <SelectTrigger size='sm' className='w-[150px]'>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value='auto'>{t('proxies.proxyColsAuto')}</SelectItem>
                <SelectItem value='1'>{t('proxies.proxyCols1')}</SelectItem>
                <SelectItem value='2'>{t('proxies.proxyCols2')}</SelectItem>
                <SelectItem value='3'>{t('proxies.proxyCols3')}</SelectItem>
                <SelectItem value='4'>{t('proxies.proxyCols4')}</SelectItem>
              </SelectContent>
            </Select>
          </SettingItem>
          <SettingItem title={t('proxies.nodeSortMethod')} divider>
            <Tabs
              value={proxyDisplayOrder}
              onValueChange={async value => {
                await patchAppConfig({
                  proxyDisplayOrder: value as 'default' | 'delay' | 'name',
                });
              }}
            >
              <TabsList>
                <TabsTrigger value='default'>{t('proxies.sortDefault')}</TabsTrigger>
                <TabsTrigger value='delay'>{t('proxies.sortDelay')}</TabsTrigger>
                <TabsTrigger value='name'>{t('proxies.sortName')}</TabsTrigger>
              </TabsList>
            </Tabs>
          </SettingItem>
          <SettingItem title={t('proxies.proxyGroupDetails')} divider>
            <Tabs
              value={groupDisplayLayout}
              onValueChange={async value => {
                await patchAppConfig({
                  groupDisplayLayout: value as 'hidden' | 'single' | 'double',
                });
              }}
            >
              <TabsList>
                <TabsTrigger value='hidden'>{t('proxies.displayHidden')}</TabsTrigger>
                <TabsTrigger value='single'>{t('proxies.displaySingle')}</TabsTrigger>
                <TabsTrigger value='double'>{t('proxies.displayDouble')}</TabsTrigger>
              </TabsList>
            </Tabs>
          </SettingItem>
          <SettingItem title={t('proxies.proxyNodeDetails')} divider>
            <Tabs
              value={proxyDisplayLayout}
              onValueChange={async value => {
                await patchAppConfig({
                  proxyDisplayLayout: value as 'hidden' | 'single' | 'double',
                });
              }}
            >
              <TabsList>
                <TabsTrigger value='hidden'>{t('proxies.displayHidden')}</TabsTrigger>
                <TabsTrigger value='single'>{t('proxies.displaySingle')}</TabsTrigger>
                <TabsTrigger value='double'>{t('proxies.displayDouble')}</TabsTrigger>
              </TabsList>
            </Tabs>
          </SettingItem>
          <SettingItem title={t('proxies.disconnectOnSwitch')} divider>
            <Switch
              checked={autoCloseConnection}
              onCheckedChange={value => {
                patchAppConfig({ autoCloseConnection: value });
              }}
            />
          </SettingItem>
          <SettingItem title={t('proxies.expandProxyGroups')} divider>
            <Switch
              checked={expandProxyGroups}
              onCheckedChange={value => {
                patchAppConfig({ expandProxyGroups: value });
              }}
            />
          </SettingItem>
          <SettingItem title={t('proxies.delayTestUrl')} divider>
            <Input
              className='w-[60%] h-8'
              value={url}
              placeholder={t('proxies.delayTestUrlPlaceholder')}
              onChange={event => {
                const value = event.target.value;
                setUrl(value);
                setUrlDebounce(value);
              }}
            />
          </SettingItem>
          <SettingItem title={t('proxies.delayTestConcurrency')} divider>
            <Input
              type='number'
              className='w-[100px] h-8'
              value={delayTestConcurrency?.toString() ?? ''}
              placeholder={t('proxies.delayTestConcurrencyPlaceholder')}
              onChange={event => {
                patchAppConfig({ delayTestConcurrency: parseInt(event.target.value, 10) });
              }}
            />
          </SettingItem>
          <SettingItem title={t('proxies.delayTestTimeout')}>
            <Input
              type='number'
              className='w-[100px] h-8'
              value={delayTestTimeout?.toString() ?? ''}
              placeholder={t('proxies.delayTestTimeoutPlaceholder')}
              onChange={event => {
                patchAppConfig({ delayTestTimeout: parseInt(event.target.value, 10) });
              }}
            />
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

export default ProxySettingModal;
