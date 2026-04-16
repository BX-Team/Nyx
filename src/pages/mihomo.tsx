import { CloudDownload } from 'lucide-react';
import PubSub from 'pubsub-js';
import type React from 'react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import BasePage from '@/components/base/base-page';
import SettingCard from '@/components/base/base-setting-card';
import SettingItem from '@/components/base/base-setting-item';
import AdvancedSetting from '@/components/mihomo/advanced-settings';
import ControllerSetting from '@/components/mihomo/controller-setting';
import EnvSetting from '@/components/mihomo/env-setting';
import PortSetting from '@/components/mihomo/port-setting';
import ServiceModal from '@/components/mihomo/service-modal';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Spinner } from '@/components/ui/spinner';
import { Switch } from '@/components/ui/switch';
import { useAppConfig } from '@/hooks/use-app-config';
import { useControledMihomoConfig } from '@/hooks/use-controled-mihomo-config';
import { useProfileConfig } from '@/hooks/use-profile-config';
import {
  findSystemMihomo,
  initService,
  installService,
  mihomoUpgrade,
  restartCore,
  restartService,
  startService,
  stopService,
  uninstallService,
} from '@/utils/ipc';

let systemCorePathsCache: string[] | null = null;
let cachePromise: Promise<string[]> | null = null;

const getSystemCorePaths = async (): Promise<string[]> => {
  if (systemCorePathsCache !== null) return systemCorePathsCache;
  if (cachePromise !== null) return cachePromise;

  cachePromise = findSystemMihomo()
    .then(paths => {
      systemCorePathsCache = paths;
      cachePromise = null;
      return paths;
    })
    .catch(() => {
      cachePromise = null;
      return [];
    });

  return cachePromise;
};

getSystemCorePaths().catch(() => {});

const Mihomo: React.FC = () => {
  const { t } = useTranslation();
  const { appConfig, patchAppConfig } = useAppConfig();
  const { core = 'mihomo', maxLogDays = 7 } = appConfig || {};
  const { controledMihomoConfig, patchControledMihomoConfig } = useControledMihomoConfig();
  const { profileConfig } = useProfileConfig();
  const hasProfiles = (profileConfig?.items?.length ?? 0) > 0;
  const { ipv6, 'log-level': logLevel = 'info' } = controledMihomoConfig || {};

  const [upgrading, setUpgrading] = useState(false);
  const [showServiceModal, setShowServiceModal] = useState(false);
  const [systemCorePaths, setSystemCorePaths] = useState<string[]>(systemCorePathsCache || []);
  const [loadingPaths, setLoadingPaths] = useState(systemCorePathsCache === null);

  useEffect(() => {
    if (systemCorePathsCache !== null) return;

    getSystemCorePaths()
      .then(setSystemCorePaths)
      .catch(() => {})
      .finally(() => setLoadingPaths(false));
  }, []);

  useEffect(() => {
    const token = PubSub.subscribe('open-service-modal', () => {
      setShowServiceModal(true);
    });
    return () => {
      PubSub.unsubscribe(token);
    };
  }, []);

  const onChangeNeedRestart = async (patch: Partial<MihomoConfig>): Promise<void> => {
    await patchControledMihomoConfig(patch);
    await restartCore();
  };

  const handleConfigChangeWithRestart = async (key: string, value: unknown): Promise<void> => {
    try {
      await patchAppConfig({ [key]: value });
      await restartCore();
      PubSub.publish('mihomo-core-changed');
    } catch (e) {
      toast.error(`${e}`);
    }
  };

  const handleCoreUpgrade = async (): Promise<void> => {
    try {
      setUpgrading(true);
      await mihomoUpgrade(core);
      setTimeout(() => PubSub.publish('mihomo-core-changed'), 2000);
    } catch (e) {
      if (typeof e === 'string' && e.includes('already using latest version')) {
        toast.info(t('pages.mihomo.alreadyLatest'));
      } else {
        toast.error(`${e}`);
      }
    } finally {
      setUpgrading(false);
    }
  };

  const handleCoreChange = async (newCore: 'mihomo' | 'mihomo-alpha' | 'system'): Promise<void> => {
    if (newCore === 'system') {
      const paths = await getSystemCorePaths();

      if (paths.length === 0) {
        toast.error(t('pages.mihomo.systemCoreNotFound'));
        return;
      }

      if (!appConfig?.systemCorePath || !paths.includes(appConfig.systemCorePath)) {
        await patchAppConfig({ systemCorePath: paths[0] });
      }
    }
    handleConfigChangeWithRestart('core', newCore);
  };

  return (
    <BasePage title={t('pages.mihomo.title')}>
      {showServiceModal && (
        <ServiceModal
          onChange={setShowServiceModal}
          onInit={async () => {
            if (!hasProfiles) {
              toast.warning(t('mihomo.serviceModal.noProfilesWarning'));
              return;
            }
            await initService();
            toast.success(t('pages.mihomo.serviceInitSuccess'));
          }}
          onInstall={async () => {
            await installService();
            toast.success(t('pages.mihomo.serviceInstallSuccess'));
            if (hasProfiles) {
              try {
                await initService();
                toast.success(t('pages.mihomo.serviceInitSuccess'));
              } catch (e) {
                toast.error(`${e}`);
              }
            }
          }}
          onUninstall={async () => {
            await uninstallService();
            toast.success(t('pages.mihomo.serviceUninstallSuccess'));
          }}
          onStart={async () => {
            if (!hasProfiles) {
              toast.warning(t('mihomo.serviceModal.noProfilesWarning'));
              return;
            }
            await startService();
            toast.success(t('pages.mihomo.serviceStartSuccess'));
          }}
          onRestart={async () => {
            await restartService();
            toast.success(t('pages.mihomo.serviceRestartSuccess'));
          }}
          onStop={async () => {
            await stopService();
            toast.success(t('pages.mihomo.serviceStopSuccess'));
          }}
        />
      )}
      <SettingCard>
        <SettingItem
          title={t('pages.mihomo.coreVersion')}
          actions={
            core === 'mihomo' || core === 'mihomo-alpha' ? (
              <Button
                size='icon-sm'
                title={t('pages.mihomo.upgradeCore')}
                variant='ghost'
                disabled={upgrading}
                aria-busy={upgrading}
                onClick={handleCoreUpgrade}
              >
                {upgrading ? <Spinner className='size-4' /> : <CloudDownload className='text-lg' />}
              </Button>
            ) : null
          }
          divider
        >
          <Select value={core} onValueChange={value => handleCoreChange(value as 'mihomo' | 'mihomo-alpha' | 'system')}>
            <SelectTrigger size='sm' className='w-[300px]'>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value='mihomo'>{t('pages.mihomo.builtinStable')}</SelectItem>
              <SelectItem value='mihomo-alpha'>{t('pages.mihomo.builtinPreview')}</SelectItem>
            </SelectContent>
          </Select>
        </SettingItem>
        {core === 'system' && (
          <SettingItem title={t('pages.mihomo.systemCorePath')} divider>
            <Select
              value={appConfig?.systemCorePath}
              disabled={loadingPaths}
              onValueChange={value => {
                if (value) handleConfigChangeWithRestart('systemCorePath', value);
              }}
            >
              <SelectTrigger size='sm' className='w-[350px]'>
                <SelectValue
                  placeholder={loadingPaths ? t('pages.mihomo.searchingCore') : t('pages.mihomo.coreNotFound')}
                />
              </SelectTrigger>
              <SelectContent>
                {loadingPaths ? (
                  <SelectItem value=''>{t('pages.mihomo.searchingCore')}</SelectItem>
                ) : systemCorePaths.length > 0 ? (
                  systemCorePaths.map(path => (
                    <SelectItem key={path} value={path}>
                      {path}
                    </SelectItem>
                  ))
                ) : (
                  <SelectItem value=''>{t('pages.mihomo.coreNotFound')}</SelectItem>
                )}
              </SelectContent>
            </Select>
            {!loadingPaths && systemCorePaths.length === 0 && (
              <div className='mt-2 text-sm text-warning'>{t('pages.mihomo.coreNotFoundWarning')}</div>
            )}
          </SettingItem>
        )}
        <SettingItem title={t('pages.mihomo.serviceStatus')} divider>
          <Button size='sm' onClick={() => setShowServiceModal(true)}>
            {t('pages.mihomo.manage')}
          </Button>
        </SettingItem>
        <SettingItem title='IPv6' divider>
          <Switch checked={ipv6 ?? false} onCheckedChange={v => onChangeNeedRestart({ ipv6: v })} />
        </SettingItem>
        <SettingItem title={t('pages.mihomo.logRetentionDays')} divider>
          <Input
            type='number'
            className='h-8 w-[100px]'
            value={maxLogDays.toString()}
            onChange={event => patchAppConfig({ maxLogDays: parseInt(event.target.value, 10) })}
          />
        </SettingItem>
        <SettingItem title={t('pages.mihomo.logLevel')}>
          <Select value={logLevel} onValueChange={value => onChangeNeedRestart({ 'log-level': value as LogLevel })}>
            <SelectTrigger size='sm' className='w-[100px]'>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value='silent'>{t('pages.mihomo.silent')}</SelectItem>
              <SelectItem value='error'>{t('pages.mihomo.error')}</SelectItem>
              <SelectItem value='warning'>{t('pages.mihomo.warning')}</SelectItem>
              <SelectItem value='info'>{t('pages.mihomo.info')}</SelectItem>
              <SelectItem value='debug'>{t('pages.mihomo.debug')}</SelectItem>
            </SelectContent>
          </Select>
        </SettingItem>
      </SettingCard>
      <PortSetting />
      <ControllerSetting />
      <EnvSetting />
      <AdvancedSetting />
    </BasePage>
  );
};

export default Mihomo;
