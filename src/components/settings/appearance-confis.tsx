import type React from 'react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Switch } from '@/components/ui/switch';
import { useAppConfig } from '@/hooks/use-app-config';
import { platform } from '@/utils/init';
import { closeTrayIcon, isAlwaysOnTop, setAlwaysOnTop, showTrayIcon, updateTrayIcon } from '@/utils/ipc';
import SettingCard from '../base/base-setting-card';
import SettingItem from '../base/base-setting-item';

interface AppearanceConfigProps {
  showHiddenSettings: boolean;
}

const AppearanceConfig: React.FC<AppearanceConfigProps> = () => {
  const { t } = useTranslation();
  const { appConfig, patchAppConfig } = useAppConfig();
  const { proxyInTray = true, disableTray = false, alwaysOnTop: savedAlwaysOnTop = false } = appConfig || {};
  const [onTop, setOnTop] = useState(savedAlwaysOnTop);

  useEffect(() => {
    isAlwaysOnTop().then(setOnTop);
  }, []);

  return (
    <SettingCard title={t('settings.appearance.title')}>
      <SettingItem title={t('settings.appearance.disableTrayIcon')} divider>
        <Switch
          checked={disableTray}
          onCheckedChange={async value => {
            await patchAppConfig({ disableTray: value });
            if (value) {
              closeTrayIcon();
            } else {
              showTrayIcon();
            }
          }}
        />
      </SettingItem>
      {platform !== 'linux' && (
        <SettingItem title={t('settings.appearance.trayShowNodeInfo')} divider>
          <Switch
            checked={proxyInTray}
            onCheckedChange={async value => {
              await patchAppConfig({ proxyInTray: value });
              await updateTrayIcon();
            }}
          />
        </SettingItem>
      )}
      <SettingItem title={t('settings.appearance.alwaysOnTop')}>
        <Switch
          checked={onTop}
          onCheckedChange={async value => {
            await setAlwaysOnTop(value);
            await patchAppConfig({ alwaysOnTop: value });
            setOnTop(await isAlwaysOnTop());
          }}
        />
      </SettingItem>
    </SettingCard>
  );
};

export default AppearanceConfig;
