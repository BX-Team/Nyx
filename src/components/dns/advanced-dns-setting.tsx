import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Switch } from '@/components/ui/switch';
import { isValidDnsServer, isValidDomainWildcard } from '@/utils/validate';
import EditableList from '../base/base-list-editor';
import SettingCard from '../base/base-setting-card';
import SettingItem from '../base/base-setting-item';

interface AdvancedDnsSettingProps {
  respectRules: boolean;
  directNameserver: string[];
  proxyServerNameserver: string[];
  nameserverPolicy: Record<string, string | string[]>;
  hosts?: IHost[];
  useHosts: boolean;
  useSystemHosts: boolean;
  onRespectRulesChange: (v: boolean) => void;
  onDirectNameserverChange: (list: string[]) => void;
  onProxyNameserverChange: (list: string[]) => void;
  onNameserverPolicyChange: (policy: Record<string, string | string[]>) => void;
  onUseSystemHostsChange: (v: boolean) => void;
  onUseHostsChange: (v: boolean) => void;
  onHostsChange: (hosts: IHost[]) => void;
  onErrorChange?: (hasError: boolean) => void;
}

const AdvancedDnsSetting: React.FC<AdvancedDnsSettingProps> = ({
  respectRules,
  directNameserver,
  proxyServerNameserver,
  nameserverPolicy,
  hosts,
  useHosts,
  useSystemHosts,
  onRespectRulesChange,
  onDirectNameserverChange,
  onProxyNameserverChange,
  onNameserverPolicyChange,
  onUseSystemHostsChange,
  onUseHostsChange,
  onHostsChange,
  onErrorChange,
}) => {
  const { t } = useTranslation();
  const [directNameserverError, setDirectNameserverError] = useState<string | null>(null);
  const [proxyNameserverError, setProxyNameserverError] = useState<string | null>(null);
  const [nameserverPolicyError, setNameserverPolicyError] = useState<string | null>(null);
  const [hostsError, setHostsError] = useState<string | null>(null);

  React.useEffect(() => {
    const hasError = Boolean(directNameserverError || proxyNameserverError || nameserverPolicyError || hostsError);
    onErrorChange?.(hasError);
  }, [directNameserverError, proxyNameserverError, nameserverPolicyError, hostsError, onErrorChange]);

  return (
    <SettingCard title={t('dns.moreSettings')}>
      <SettingItem title={t('dns.connectionRespectRules')} divider>
        <Switch
          checked={respectRules}
          disabled={proxyServerNameserver.length === 0}
          onCheckedChange={onRespectRulesChange}
        />
      </SettingItem>
      <EditableList
        title={t('dns.directResolver')}
        items={directNameserver}
        validate={part => isValidDnsServer(part as string)}
        onChange={list => {
          const arr = list as string[];
          onDirectNameserverChange(arr);
          const firstInvalid = arr.find(f => !isValidDnsServer(f).ok);
          setDirectNameserverError(
            firstInvalid ? (isValidDnsServer(firstInvalid).error ?? t('common.formatError')) : null,
          );
        }}
        placeholder={t('pages.dns.placeholderTLS')}
      />
      <EditableList
        title={t('dns.proxyNodeResolver')}
        items={proxyServerNameserver}
        validate={part => isValidDnsServer(part as string)}
        onChange={list => {
          const arr = list as string[];
          onProxyNameserverChange(arr);
          const firstInvalid = arr.find(f => !isValidDnsServer(f).ok);
          setProxyNameserverError(
            firstInvalid ? (isValidDnsServer(firstInvalid).error ?? t('common.formatError')) : null,
          );
        }}
        placeholder={t('pages.dns.placeholderTLS')}
      />

      <EditableList
        title={t('dns.domainResolutionPolicy')}
        items={nameserverPolicy}
        validatePart1={part1 => isValidDomainWildcard(part1)}
        validatePart2={part2 => {
          const parts = part2
            .split(',')
            .map(p => p.trim())
            .filter(Boolean);
          for (const p of parts) {
            const result = isValidDnsServer(p);
            if (!result.ok) {
              return result;
            }
          }
          return { ok: true };
        }}
        onChange={newValue => {
          onNameserverPolicyChange(newValue as Record<string, string | string[]>);
          try {
            const rec = newValue as Record<string, string | string[]>;
            for (const domain of Object.keys(rec)) {
              if (!isValidDomainWildcard(domain).ok) {
                setNameserverPolicyError(isValidDomainWildcard(domain).error ?? t('dns.domainFormatError'));
                return;
              }
            }
            for (const v of Object.values(rec)) {
              if (Array.isArray(v)) {
                for (const vv of v) {
                  if (!isValidDnsServer(vv).ok) {
                    setNameserverPolicyError(isValidDnsServer(vv).error ?? t('common.formatError'));
                    return;
                  }
                }
              } else {
                const parts = (v as string)
                  .split(',')
                  .map(p => p.trim())
                  .filter(Boolean);
                for (const p of parts) {
                  if (!isValidDnsServer(p).ok) {
                    setNameserverPolicyError(isValidDnsServer(p).error ?? t('common.formatError'));
                    return;
                  }
                }
              }
            }
            setNameserverPolicyError(null);
          } catch (_e) {
            setNameserverPolicyError(t('dns.policyFormatError'));
          }
        }}
        placeholder={t('dns.domain')}
        part2Placeholder={t('dns.dnsServerCommaSeparated')}
        objectMode='record'
      />
      <SettingItem title={t('dns.useSystemHosts')} divider>
        <Switch checked={useSystemHosts} onCheckedChange={onUseSystemHostsChange} />
      </SettingItem>
      <SettingItem title={t('dns.customHosts')}>
        <Switch checked={useHosts} onCheckedChange={onUseHostsChange} />
      </SettingItem>
      {useHosts && (
        <EditableList
          items={hosts ? Object.fromEntries(hosts.map(h => [h.domain, h.value])) : {}}
          validatePart1={part1 => isValidDomainWildcard(part1)}
          onChange={rec => {
            const hostArr: IHost[] = Object.entries(rec as Record<string, string | string[]>).map(
              ([domain, value]) => ({
                domain,
                value: value as string | string[],
              }),
            );
            onHostsChange(hostArr);
            for (const domain of Object.keys(rec as Record<string, string | string[]>)) {
              if (!isValidDomainWildcard(domain).ok) {
                setHostsError(isValidDomainWildcard(domain).error ?? t('dns.domainFormatError'));
                return;
              }
            }
            setHostsError(null);
          }}
          placeholder={t('dns.domain')}
          part2Placeholder={t('dns.domainOrIPCommaSeparated')}
          objectMode='record'
          divider={false}
        />
      )}
    </SettingCard>
  );
};

export default AdvancedDnsSetting;
