import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { useAppConfig } from '@/hooks/use-app-config';
import { platform } from '@/utils/init';
import { restartCore } from '@/utils/ipc';
import EditableList from '../base/base-list-editor';
import SettingCard from '../base/base-setting-card';
import SettingItem from '../base/base-setting-item';

const EnvSetting: React.FC = () => {
  const { t } = useTranslation();
  const { appConfig, patchAppConfig } = useAppConfig();
  const { disableLoopbackDetector, disableEmbedCA, disableSystemCA, disableNftables, safePaths = [] } = appConfig || {};
  const handleConfigChangeWithRestart = async (key: string, value: unknown): Promise<void> => {
    try {
      await patchAppConfig({ [key]: value });
      await restartCore();
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      PubSub.publish('mihomo-core-changed');
    }
  };
  const [safePathsInput, setSafePathsInput] = useState(safePaths);

  return (
    <SettingCard title={t('mihomo.envSettings.environmentVariables')}>
      <SettingItem title={t('mihomo.envSettings.disableSystemCA')} divider>
        <Switch
          checked={disableSystemCA}
          onCheckedChange={v => {
            handleConfigChangeWithRestart('disableSystemCA', v);
          }}
        />
      </SettingItem>
      <SettingItem title={t('mihomo.envSettings.disableBuiltinCA')} divider>
        <Switch
          checked={disableEmbedCA}
          onCheckedChange={v => {
            handleConfigChangeWithRestart('disableEmbedCA', v);
          }}
        />
      </SettingItem>
      <SettingItem title={t('mihomo.envSettings.disableLoopbackDetection')} divider>
        <Switch
          checked={disableLoopbackDetector}
          onCheckedChange={v => {
            handleConfigChangeWithRestart('disableLoopbackDetector', v);
          }}
        />
      </SettingItem>
      {platform === 'linux' && (
        <SettingItem title={t('mihomo.envSettings.disableNftables')} divider>
          <Switch
            checked={disableNftables}
            onCheckedChange={v => {
              handleConfigChangeWithRestart('disableNftables', v);
            }}
          />
        </SettingItem>
      )}
      <SettingItem title={t('mihomo.envSettings.trustedPath')}>
        {safePathsInput.join('') !== safePaths.join('') && (
          <Button
            size='sm'
            onClick={() => {
              handleConfigChangeWithRestart('safePaths', safePathsInput);
            }}
          >
            {t('common.confirm')}
          </Button>
        )}
      </SettingItem>
      <EditableList
        items={safePathsInput}
        onChange={items => setSafePathsInput(items as string[])}
        divider={false}
      />{' '}
    </SettingCard>
  );
};

export default EnvSetting;
