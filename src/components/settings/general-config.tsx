import type React from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import useSWR from 'swr';
import { Switch } from '@/components/ui/switch';
import { useAppConfig } from '@/hooks/use-app-config';
import { checkAutoRun, disableAutoRun, enableAutoRun } from '@/utils/ipc';
import SettingCard from '../base/base-setting-card';
import SettingItem from '../base/base-setting-item';

interface GeneralConfigProps {
  showHiddenSettings: boolean;
}

const GeneralConfig: React.FC<GeneralConfigProps> = () => {
  const { t } = useTranslation();
  const { data: enable, mutate: mutateEnable } = useSWR('checkAutoRun', checkAutoRun);
  const { appConfig, patchAppConfig } = useAppConfig();
  const { silentStart = false, autoCheckUpdate } = appConfig || {};

  return (
    <SettingCard>
      <SettingItem title={t('settings.general.autoStart')} divider>
        <Switch
          checked={enable}
          onCheckedChange={async value => {
            try {
              if (value) {
                await enableAutoRun();
              } else {
                await disableAutoRun();
              }
            } catch (e) {
              toast.error(`${e}`);
            } finally {
              mutateEnable();
            }
          }}
        />
      </SettingItem>
      <SettingItem title={t('settings.general.silentStart')} divider>
        <Switch
          checked={silentStart}
          onCheckedChange={value => {
            patchAppConfig({ silentStart: value });
          }}
        />
      </SettingItem>
      <SettingItem title={t('settings.general.autoCheckUpdate')}>
        <Switch
          checked={autoCheckUpdate}
          onCheckedChange={value => {
            patchAppConfig({ autoCheckUpdate: value });
          }}
        />
      </SettingItem>
    </SettingCard>
  );
};

export default GeneralConfig;
