import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import EditableList from '@/components/base/base-list-editor';
import BasePage from '@/components/base/base-page';
import SettingCard from '@/components/base/base-setting-card';
import SettingItem from '@/components/base/base-setting-item';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Spinner } from '@/components/ui/spinner';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { useAppConfig } from '@/hooks/use-app-config';
import { useControledMihomoConfig } from '@/hooks/use-controled-mihomo-config';
import { platform } from '@/utils/init';
import { restartCore, setupFirewall } from '@/utils/ipc';

const Tun: React.FC = () => {
  const { t } = useTranslation();
  const { controledMihomoConfig, patchControledMihomoConfig } = useControledMihomoConfig();
  const { appConfig, patchAppConfig } = useAppConfig();
  const { controlTun = false } = appConfig || {};
  const { tun } = controledMihomoConfig || {};
  const [loading, setLoading] = useState(false);
  const {
    device = 'mihomo',
    stack = 'mixed',
    'auto-route': autoRoute = true,
    'auto-redirect': autoRedirect = false,
    'auto-detect-interface': autoDetectInterface = true,
    'dns-hijack': dnsHijack = ['any:53'],
    'route-exclude-address': routeExcludeAddress = [],
    'strict-route': strictRoute = false,
    'disable-icmp-forwarding': disableIcmpForwarding = false,
    mtu = 1500,
  } = tun || {};
  const [changed, setChanged] = useState(false);
  const [values, originSetValues] = useState({
    device,
    stack,
    autoRoute,
    autoRedirect,
    autoDetectInterface,
    dnsHijack,
    strictRoute,
    routeExcludeAddress,
    disableIcmpForwarding,
    mtu,
  });
  const setValues = (v: typeof values): void => {
    originSetValues(v);
    setChanged(true);
  };

  const onSave = async (patch: Partial<MihomoConfig>): Promise<void> => {
    await patchControledMihomoConfig(patch);
    await restartCore();
    setChanged(false);
  };

  return (
    <BasePage
      title={t('pages.tun.title')}
      header={
        changed && (
          <Button
            size='sm'
            className='app-nodrag'
            onClick={() =>
              onSave({
                tun: {
                  device: values.device,
                  stack: values.stack,
                  'auto-route': values.autoRoute,
                  'auto-redirect': values.autoRedirect,
                  'auto-detect-interface': values.autoDetectInterface,
                  'dns-hijack': values.dnsHijack,
                  'strict-route': values.strictRoute,
                  'route-exclude-address': values.routeExcludeAddress,
                  'disable-icmp-forwarding': values.disableIcmpForwarding,
                  mtu: values.mtu,
                },
              })
            }
          >
            {t('common.save')}
          </Button>
        )
      }
    >
      <SettingCard className='tun-settings'>
        <SettingItem title={t('pages.tun.takeOverTun')} divider>
          <Switch
            checked={controlTun}
            onCheckedChange={async value => {
              try {
                await patchAppConfig({ controlTun: value });
                await patchControledMihomoConfig(value ? {} : { tun: { enable: false } });
                await restartCore();
              } catch (e) {
                toast.error(`${e}`);
              }
            }}
          />
        </SettingItem>
        {platform === 'win32' && (
          <SettingItem title={t('pages.tun.resetFirewall')} divider>
            <Button
              size='sm'
              disabled={loading}
              onClick={async () => {
                setLoading(true);
                try {
                  await setupFirewall();
                  toast.success(t('pages.tun.firewallResetSuccess'));
                  await restartCore();
                } catch (e) {
                  toast.error(`${e}`);
                } finally {
                  setLoading(false);
                }
              }}
            >
              {loading && <Spinner className='mr-2 size-4' />}
              {t('pages.tun.resetFirewallButton')}
            </Button>
          </SettingItem>
        )}
        <SettingItem title={t('pages.tun.tunModeStack')} divider>
          <Tabs value={values.stack} onValueChange={value => setValues({ ...values, stack: value as TunStack })}>
            <TabsList>
              <TabsTrigger value='gvisor'>gVisor</TabsTrigger>
              <TabsTrigger value='mixed'>Mixed</TabsTrigger>
              <TabsTrigger value='system'>System</TabsTrigger>
            </TabsList>
          </Tabs>
        </SettingItem>
        <SettingItem title={t('pages.tun.tunCardName')} divider>
          <Input
            className='w-[100px]'
            value={values.device || ''}
            onChange={event => {
              setValues({ ...values, device: event.target.value });
            }}
          />
        </SettingItem>
        <SettingItem title={t('pages.tun.strictRoute')} divider>
          <Switch
            checked={values.strictRoute}
            onCheckedChange={value => {
              setValues({ ...values, strictRoute: value });
            }}
          />
        </SettingItem>
        <SettingItem title={t('pages.tun.autoSetRouteRules')} divider>
          <Switch
            checked={values.autoRoute}
            onCheckedChange={value => {
              setValues({ ...values, autoRoute: value });
            }}
          />
        </SettingItem>
        {platform === 'linux' && (
          <SettingItem title={t('pages.tun.autoSetTCPRedirect')} divider>
            <Switch
              checked={values.autoRedirect}
              onCheckedChange={value => {
                setValues({ ...values, autoRedirect: value });
              }}
            />
          </SettingItem>
        )}
        <SettingItem title={t('pages.tun.autoSelectTrafficExit')} divider>
          <Switch
            checked={values.autoDetectInterface}
            onCheckedChange={value => {
              setValues({ ...values, autoDetectInterface: value });
            }}
          />
        </SettingItem>
        <SettingItem title={t('pages.tun.icmpForwarding')} divider>
          <Switch
            checked={!values.disableIcmpForwarding}
            onCheckedChange={value => {
              setValues({ ...values, disableIcmpForwarding: !value });
            }}
          />
        </SettingItem>
        <SettingItem title='MTU' divider>
          <Input
            type='number'
            className='w-[100px]'
            value={values.mtu.toString()}
            onChange={event => {
              setValues({ ...values, mtu: parseInt(event.target.value, 10) });
            }}
          />
        </SettingItem>
        <SettingItem title={t('pages.tun.dnsHijack')} divider>
          <Input
            className='w-[50%]'
            value={values.dnsHijack.join(',')}
            onChange={event => {
              const inputValue = event.target.value;
              const arr = inputValue !== '' ? inputValue.split(',') : [];
              setValues({ ...values, dnsHijack: arr });
            }}
          />
        </SettingItem>
        <EditableList
          title={t('pages.tun.excludeCustomNetworks')}
          items={values.routeExcludeAddress}
          placeholder={t('pages.tun.exampleNetwork')}
          onChange={list => setValues({ ...values, routeExcludeAddress: list as string[] })}
          divider={false}
        />
      </SettingCard>
    </BasePage>
  );
};

export default Tun;
