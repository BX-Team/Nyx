import { Settings } from 'lucide-react';
import PubSub from 'pubsub-js';
import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import ConfirmModal from '@/components/base/base-confirm';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { useAppConfig } from '@/hooks/use-app-config';
import { useControledMihomoConfig } from '@/hooks/use-controled-mihomo-config';
import { restartCore, serviceStatus, triggerSysProxy, updateTrayIcon } from '@/utils/ipc';
import SettingCard from '../base/base-setting-card';
import SettingItem from '../base/base-setting-item';

const ProxySwitches: React.FC = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { controledMihomoConfig, patchControledMihomoConfig } = useControledMihomoConfig();
  const { tun } = controledMihomoConfig || {};
  const { appConfig, patchAppConfig } = useAppConfig();
  const { sysProxy, onlyActiveDevice = false } = appConfig || {};
  const { enable: sysProxyEnable, mode } = sysProxy || {};
  const { 'mixed-port': mixedPort } = controledMihomoConfig || {};
  const sysProxyDisabled = mixedPort === 0;
  const [showInstallServicePrompt, setShowInstallServicePrompt] = useState(false);

  return (
    <>
      {showInstallServicePrompt && (
        <ConfirmModal
          onChange={setShowInstallServicePrompt}
          title={t('pages.home.tunServiceInstallTitle')}
          description={t('pages.home.tunServiceInstallDescription')}
          confirmText={t('pages.home.tunServiceInstallAction')}
          onConfirm={async () => {
            navigate('/mihomo');
            setTimeout(() => {
              PubSub.publish('open-service-modal');
            }, 100);
          }}
        />
      )}
      <SettingCard>
        <SettingItem
          title={t('sider.virtualInterface')}
          actions={
            <Button size='icon-sm' variant='ghost' onClick={() => navigate('/tun')}>
              <Settings className='text-lg' />
            </Button>
          }
          divider
        >
          <Switch
            checked={tun?.enable ?? false}
            onCheckedChange={async (enable: boolean) => {
              if (enable) {
                try {
                  const status = await serviceStatus();
                  if (status === 'not-installed') {
                    setShowInstallServicePrompt(true);
                    return;
                  }
                } catch {}
                await patchControledMihomoConfig({ tun: { enable }, dns: { enable: true } });
              } else {
                await patchControledMihomoConfig({ tun: { enable } });
              }
              await restartCore();
              await updateTrayIcon();
            }}
          />
        </SettingItem>
        <SettingItem
          title={t('sider.systemProxy')}
          actions={
            <Button size='icon-sm' variant='ghost' onClick={() => navigate('/sysproxy')}>
              <Settings className='text-lg' />
            </Button>
          }
        >
          <Switch
            checked={sysProxyEnable ?? false}
            disabled={mode === 'manual' && sysProxyDisabled}
            onCheckedChange={async (enable: boolean) => {
              if (mode === 'manual' && sysProxyDisabled) return;
              try {
                await triggerSysProxy(enable, onlyActiveDevice);
                await patchAppConfig({ sysProxy: { enable } });
                await updateTrayIcon();
              } catch (e) {
                toast.error(`${e}`);
              }
            }}
          />
        </SettingItem>
      </SettingCard>
    </>
  );
};

export default ProxySwitches;
