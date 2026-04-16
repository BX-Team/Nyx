import type React from 'react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import EditableList from '@/components/base/base-list-editor';
import BasePage from '@/components/base/base-page';
import SettingCard from '@/components/base/base-setting-card';
import SettingItem from '@/components/base/base-setting-item';
import ByPassEditorModal from '@/components/sysproxy/bypass-editor-modal';
import PacEditorModal from '@/components/sysproxy/pac-editor-modal';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useAppConfig } from '@/hooks/use-app-config';
import { platform } from '@/utils/init';
import { openUWPTool, triggerSysProxy } from '@/utils/ipc';

const defaultPacScript = `
function FindProxyForURL(url, host) {
  return "PROXY 127.0.0.1:%mixed-port%; SOCKS5 127.0.0.1:%mixed-port%; DIRECT;";
}
`;

const Sysproxy: React.FC = () => {
  const { t } = useTranslation();
  const defaultBypass: string[] =
    platform === 'linux'
      ? ['localhost', '.local', '127.0.0.1/8', '192.168.0.0/16', '10.0.0.0/8', '172.16.0.0/12', '::1']
      : [
          'localhost',
          '127.*',
          '192.168.*',
          '10.*',
          '172.16.*',
          '172.17.*',
          '172.18.*',
          '172.19.*',
          '172.20.*',
          '172.21.*',
          '172.22.*',
          '172.23.*',
          '172.24.*',
          '172.25.*',
          '172.26.*',
          '172.27.*',
          '172.28.*',
          '172.29.*',
          '172.30.*',
          '172.31.*',
          '<local>',
        ];

  const { appConfig, patchAppConfig } = useAppConfig();
  const { sysProxy, affectVPNConnections = false } = appConfig || ({ sysProxy: { enable: false } } as AppConfig);
  const [changed, setChanged] = useState(false);
  const [values, originSetValues] = useState({
    enable: sysProxy.enable,
    host: sysProxy.host ?? '',
    bypass: sysProxy.bypass ?? defaultBypass,
    mode: sysProxy.mode ?? 'manual',
    pacScript: sysProxy.pacScript ?? defaultPacScript,
    settingMode: sysProxy.settingMode ?? 'exec',
  });
  useEffect(() => {
    originSetValues(prev => ({
      ...prev,
      enable: sysProxy.enable,
    }));
  }, [sysProxy.enable]);
  const [openEditor, setOpenEditor] = useState(false);
  const [openPacEditor, setOpenPacEditor] = useState(false);

  const setValues = (v: typeof values): void => {
    originSetValues(v);
    setChanged(true);
  };
  const onSave = async (): Promise<void> => {
    if (values.host && values.mode === 'manual') {
      const hostPattern = /^[\w.-]+(:\d+)?$/;
      if (!hostPattern.test(values.host)) {
        toast.error('Invalid proxy host format');
        return;
      }
    }
    values.bypass = values.bypass.filter(b => b.trim().length > 0);
    await patchAppConfig({ sysProxy: values });
    setChanged(false);
    if (values.enable) {
      try {
        await triggerSysProxy(values.enable, affectVPNConnections);
      } catch (e) {
        toast.error(`${e}`);
        await patchAppConfig({ sysProxy: { enable: false } });
      }
    }
  };

  return (
    <BasePage
      title={t('pages.sysproxy.title')}
      header={
        changed && (
          <Button className='app-nodrag' size='sm' onClick={onSave}>
            {t('common.save')}
          </Button>
        )
      }
    >
      {openPacEditor && (
        <PacEditorModal
          script={values.pacScript || defaultPacScript}
          onCancel={() => setOpenPacEditor(false)}
          onConfirm={(script: string) => {
            setValues({ ...values, pacScript: script });
            setOpenPacEditor(false);
          }}
        />
      )}
      {openEditor && (
        <ByPassEditorModal
          bypass={values.bypass}
          onCancel={() => setOpenEditor(false)}
          onConfirm={async (list: string[]) => {
            setOpenEditor(false);
            setValues({
              ...values,
              bypass: list,
            });
          }}
        />
      )}
      <SettingCard className='sysproxy-settings'>
        <SettingItem title={t('pages.sysproxy.affectVPNConnections')} divider>
          <Switch
            checked={affectVPNConnections}
            onCheckedChange={async value => {
              await patchAppConfig({ affectVPNConnections: value });
              if (sysProxy.enable) {
                try {
                  await triggerSysProxy(sysProxy.enable, value);
                } catch (e) {
                  toast.error(`${e}`);
                }
              }
            }}
          />
        </SettingItem>
        <SettingItem title={t('pages.sysproxy.proxyHost')} divider>
          <Input
            className='w-[50%]'
            value={values.host}
            placeholder={t('pages.sysproxy.proxyHostPlaceholder')}
            onChange={event => {
              setValues({ ...values, host: event.target.value });
            }}
          />
        </SettingItem>
        <SettingItem title={t('pages.sysproxy.proxyMode')} divider>
          <Tabs value={values.mode} onValueChange={value => setValues({ ...values, mode: value as SysProxyMode })}>
            <TabsList>
              <TabsTrigger value='manual'>{t('pages.sysproxy.manual')}</TabsTrigger>
              <TabsTrigger value='auto'>{t('pages.sysproxy.auto')}</TabsTrigger>
            </TabsList>
          </Tabs>
        </SettingItem>
        {platform === 'win32' && (
          <SettingItem title={t('pages.sysproxy.uwpTool')} divider>
            <Button
              size='sm'
              onClick={async () => {
                await openUWPTool();
              }}
            >
              {t('pages.sysproxy.openUWPTool')}
            </Button>
          </SettingItem>
        )}
        {values.mode === 'auto' && (
          <SettingItem title={t('pages.sysproxy.proxyMode')}>
            <Button size='sm' onClick={() => setOpenPacEditor(true)}>
              {t('pages.sysproxy.editPACScript')}
            </Button>
          </SettingItem>
        )}
        {values.mode === 'manual' && (
          <>
            <SettingItem title={t('pages.sysproxy.addDefaultBypass')} divider>
              <Button
                size='sm'
                onClick={() => {
                  setValues({
                    ...values,
                    bypass: Array.from(new Set([...defaultBypass, ...values.bypass])),
                  });
                }}
              >
                {t('pages.sysproxy.addDefaultBypass')}
              </Button>
            </SettingItem>
            <SettingItem title={t('pages.sysproxy.proxyBypassList')}>
              <Button
                size='sm'
                onClick={async () => {
                  setOpenEditor(true);
                }}
              >
                {t('common.edit')}
              </Button>
            </SettingItem>
            <EditableList
              items={values.bypass}
              onChange={list => setValues({ ...values, bypass: list as string[] })}
              placeholder={t('pages.sysproxy.exampleBypass')}
              divider={false}
            />
          </>
        )}
      </SettingCard>
    </BasePage>
  );
};

export default Sysproxy;
