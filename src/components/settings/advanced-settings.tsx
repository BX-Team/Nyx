import { ChevronDownIcon, Copy, MessageCircleQuestionMark, Settings } from 'lucide-react';
import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { InputGroup, InputGroupAddon, InputGroupInput, InputGroupText } from '@/components/ui/input-group';
import { Switch } from '@/components/ui/switch';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useAppConfig } from '@/hooks/use-app-config';
import { platform } from '@/utils/init';
import {
  copyEnv,
  patchControledMihomoConfig,
  restartCore,
  startNetworkDetection,
  stopNetworkDetection,
} from '@/utils/ipc';
import EditableList from '../base/base-list-editor';
import SettingCard from '../base/base-setting-card';
import SettingItem from '../base/base-setting-item';

type EnvType = 'bash' | 'cmd' | 'powershell' | 'nushell';

const envOptions: Array<{ value: EnvType; label: string }> = [
  { value: 'bash', label: 'Bash' },
  { value: 'cmd', label: 'CMD' },
  { value: 'powershell', label: 'PowerShell' },
  { value: 'nushell', label: 'NuShell' },
];

interface AdvancedSettingsProps {
  showHiddenSettings: boolean;
}

const AdvancedSettings: React.FC<AdvancedSettingsProps> = props => {
  const { showHiddenSettings } = props;
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { appConfig, patchAppConfig } = useAppConfig();
  const {
    controlDns = true,
    controlSniff = true,
    envType = [platform === 'win32' ? 'powershell' : 'bash'],
    networkDetection = false,
    networkDetectionBypass = ['VMware', 'vEthernet'],
    networkDetectionInterval = 10,
  } = appConfig || {};

  const [bypass, setBypass] = useState(networkDetectionBypass);
  const [interval, setInterval] = useState(networkDetectionInterval);
  const envTypeValue = envType as EnvType[];
  const envTypeLabels = envOptions.filter(option => envTypeValue.includes(option.value)).map(option => option.label);
  const envTypeLabel = envTypeLabels.length ? envTypeLabels.join(', ') : '-';

  const handleEnvTypeChange = async (value: EnvType, checked: boolean): Promise<void> => {
    const next = checked ? Array.from(new Set([...envTypeValue, value])) : envTypeValue.filter(item => item !== value);
    if (next.length === 0) return;
    try {
      await patchAppConfig({
        envType: next,
      });
    } catch (e) {
      toast.error(`${e}`);
    }
  };

  return (
    <SettingCard title={t('settings.advanced.moreSettings')}>
      {showHiddenSettings && (
        <SettingItem
          title={t('settings.advanced.copyEnvType')}
          actions={envType.map(type => (
            <Button key={type} title={type} size='icon-sm' variant='ghost' onClick={() => copyEnv(type)}>
              <Copy className='text-lg' />
            </Button>
          ))}
          divider
        >
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant='outline' size='sm' className='w-37.5 justify-between'>
                <span className='truncate'>{envTypeLabel}</span>
                <ChevronDownIcon className='size-4 opacity-50' />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent className='w-37.5'>
              {envOptions.map(option => (
                <DropdownMenuCheckboxItem
                  key={option.value}
                  checked={envTypeValue.includes(option.value)}
                  onCheckedChange={checked => handleEnvTypeChange(option.value, checked)}
                >
                  {option.label}
                </DropdownMenuCheckboxItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        </SettingItem>
      )}
      <SettingItem
        title={t('settings.advanced.takeOverDNS')}
        actions={
          <Button size='icon-sm' variant='ghost' onClick={() => navigate('/dns')}>
            <Settings className='text-lg' />
          </Button>
        }
        divider
      >
        <Switch
          checked={controlDns}
          onCheckedChange={async value => {
            try {
              await patchAppConfig({ controlDns: value });
              await patchControledMihomoConfig({});
              await restartCore();
            } catch (e) {
              toast.error(`${e}`);
            }
          }}
        />
      </SettingItem>
      <SettingItem
        title={t('settings.advanced.takeOverSniffer')}
        actions={
          <Button size='icon-sm' variant='ghost' onClick={() => navigate('/sniffer')}>
            <Settings className='text-lg' />
          </Button>
        }
        divider
      >
        <Switch
          checked={controlSniff}
          onCheckedChange={async value => {
            try {
              await patchAppConfig({ controlSniff: value });
              await patchControledMihomoConfig({});
              await restartCore();
            } catch (e) {
              toast.error(`${e}`);
            }
          }}
        />
      </SettingItem>
      <SettingItem
        title={t('settings.advanced.stopCoreOnDisconnect')}
        actions={
          <Tooltip>
            <TooltipTrigger asChild>
              <Button size='icon-sm' variant='ghost'>
                <MessageCircleQuestionMark className='text-lg' />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t('settings.advanced.stopCoreOnDisconnectHelp')}</TooltipContent>
          </Tooltip>
        }
        divider={networkDetection}
      >
        <Switch
          checked={networkDetection}
          onCheckedChange={value => {
            patchAppConfig({ networkDetection: value });
            if (value) {
              startNetworkDetection();
            } else {
              stopNetworkDetection();
            }
          }}
        />
      </SettingItem>
      {networkDetection && (
        <>
          <SettingItem title={t('settings.advanced.disconnectDetectInterval')} divider>
            <div className='flex items-center'>
              {interval !== networkDetectionInterval && (
                <Button
                  size='sm'
                  className='mr-2'
                  onClick={async () => {
                    await patchAppConfig({ networkDetectionInterval: interval });
                    await startNetworkDetection();
                  }}
                >
                  {t('common.confirm')}
                </Button>
              )}
              <InputGroup className='w-37.5 h-8'>
                <InputGroupInput
                  type='number'
                  value={interval.toString()}
                  min={1}
                  onChange={event => {
                    setInterval(parseInt(event.target.value, 10));
                  }}
                />
                <InputGroupAddon align='inline-end'>
                  <InputGroupText>{t('settings.advanced.seconds')}</InputGroupText>
                </InputGroupAddon>
              </InputGroup>
            </div>
          </SettingItem>
          <SettingItem title={t('settings.advanced.bypassDetectInterfaces')}>
            {bypass.length !== networkDetectionBypass.length && (
              <Button
                size='sm'
                onClick={async () => {
                  await patchAppConfig({ networkDetectionBypass: bypass });
                  await startNetworkDetection();
                }}
              >
                {t('common.confirm')}
              </Button>
            )}
          </SettingItem>
          <EditableList items={bypass} onChange={list => setBypass(list as string[])} divider={false} />
        </>
      )}
    </SettingCard>
  );
};

export default AdvancedSettings;
