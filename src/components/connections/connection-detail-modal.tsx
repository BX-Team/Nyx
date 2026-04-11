import dayjs from 'dayjs';
import { t } from 'i18next';
import { Check, Copy } from 'lucide-react';
import type React from 'react';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Dialog, DialogClose, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { calcTraffic } from '@/utils/calc';

interface Props {
  connection: ControllerConnectionDetail;
  onClose: () => void;
}

interface CopyableValueProps {
  label: string;
  value: string | string[];
  displayName?: string;
  prefix?: string[];
}

const getSubDomains = (domain: string): string[] =>
  domain.split('.').length <= 2
    ? [domain]
    : domain
        .split('.')
        .map((_, i, parts) => parts.slice(i).join('.'))
        .slice(0, -1);

const isIPv6 = (ip: string): boolean => ip.includes(':');

const buildMenuItems = (value: string | string[], prefix: string[], displayName?: string) => {
  const rawText = displayName || (Array.isArray(value) ? value.join(', ') : value);
  const items = [{ key: 'raw', text: rawText }];

  const buildPrefixItems = (p: string, v: string) => {
    if (!p || !v) return [];
    if (p === 'DOMAIN-SUFFIX') {
      return getSubDomains(v).map(subV => ({ key: `${p},${subV}`, text: `${p},${subV}` }));
    }
    if (p === 'IP-ASN' || p === 'SRC-IP-ASN') {
      return [{ key: `${p},${v.split(' ')[0]}`, text: `${p},${v.split(' ')[0]}` }];
    }
    const suffix = p === 'IP-CIDR' || p === 'SRC-IP-CIDR' ? (isIPv6(v) ? '/128' : '/32') : '';
    return [{ key: `${p},${v}${suffix}`, text: `${p},${v}${suffix}` }];
  };

  if (Array.isArray(value)) {
    value.forEach((v, i) => {
      const p = prefix[i];
      if (p && v) items.push(...buildPrefixItems(p, v));
    });
  } else {
    prefix.forEach(p => {
      items.push(...buildPrefixItems(p, value));
    });
  }

  return items;
};

const CopyableValue: React.FC<CopyableValueProps> = ({ label, value, displayName, prefix = [] }) => {
  const [copied, setCopied] = useState(false);
  const displayText = displayName || (Array.isArray(value) ? value.join(', ') : value);
  const menuItems = buildMenuItems(value, prefix, displayName);
  const hasMenuItems = menuItems.length > 1;

  const handleSimpleCopy = () => {
    const text = Array.isArray(value) ? value.join(', ') : value;
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className='flex items-start justify-between gap-2 py-1.5 group'>
      <span className='text-xs text-muted-foreground shrink-0 mt-0.5 min-w-[100px]'>{label}</span>
      <div className='flex items-center gap-1 min-w-0'>
        {hasMenuItems ? (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                size='icon-xs'
                variant='ghost'
                className='shrink-0 opacity-0 group-hover:opacity-100 transition-opacity size-5'
              >
                <Copy className='text-xs text-muted-foreground' />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align='end'>
              {menuItems.map(({ key, text }) => (
                <DropdownMenuItem
                  key={key}
                  onClick={() =>
                    navigator.clipboard.writeText(
                      key === 'raw' ? (Array.isArray(value) ? value.join(', ') : value) : key,
                    )
                  }
                >
                  {text}
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        ) : (
          <Button
            size='icon-xs'
            variant='ghost'
            className='shrink-0 opacity-0 group-hover:opacity-100 transition-opacity size-5'
            onClick={handleSimpleCopy}
          >
            {copied ? <Check className='text-xs text-green-500' /> : <Copy className='text-xs text-muted-foreground' />}
          </Button>
        )}
        <span className='text-sm text-right break-all'>{displayText}</span>
      </div>
    </div>
  );
};

const DetailRow: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <div className='flex items-start justify-between gap-2 py-1.5'>
    <span className='text-xs text-muted-foreground shrink-0 mt-0.5 min-w-[100px]'>{label}</span>
    <span className='text-sm text-right break-all'>{children}</span>
  </div>
);

const Section: React.FC<{ title: string; children: React.ReactNode }> = ({ title, children }) => (
  <div>
    <h4 className='text-xs font-medium text-muted-foreground/70 uppercase tracking-wider mb-1'>{title}</h4>
    <div className='rounded-lg bg-muted/30 px-3 py-1 divide-y divide-border/50'>{children}</div>
  </div>
);

const ConnectionDetailModal: React.FC<Props> = ({ connection, onClose }) => {
  const destination =
    connection.metadata.host ||
    connection.metadata.sniffHost ||
    connection.metadata.destinationIP ||
    connection.metadata.remoteDestination ||
    '';
  const processName = connection.metadata.process || connection.metadata.sourceIP || '';

  return (
    <Dialog
      open={true}
      onOpenChange={open => {
        if (!open) onClose();
      }}
    >
      <DialogContent className='max-w-md flag-emoji' showCloseButton={false}>
        <DialogHeader>
          <div className='flex items-start justify-between gap-3'>
            <div className='min-w-0'>
              <DialogTitle className='text-base leading-tight break-all'>
                {processName && (
                  <>
                    {processName} <span className='text-muted-foreground font-normal'>→</span>{' '}
                  </>
                )}
                {destination}
              </DialogTitle>
              <div className='flex items-center gap-1.5 mt-2 flex-wrap'>
                <Badge variant={connection.isActive ? 'default' : 'destructive'} className='text-[10px] px-1.5 py-0'>
                  {connection.isActive ? t('connections.detail.active') : t('connections.detail.closed')}
                </Badge>
                <Badge variant='outline' className='text-[10px] px-1.5 py-0 rounded-sm'>
                  {connection.metadata.type}({connection.metadata.network.toUpperCase()})
                </Badge>
                <span className='text-[11px] text-muted-foreground'>{dayjs(connection.start).fromNow()}</span>
              </div>
            </div>
          </div>
        </DialogHeader>

        <div className='overflow-y-auto max-h-[55vh] flex flex-col gap-3 -mx-1 px-1'>
          {/* Traffic */}
          <Section title={t('connections.sections.traffic')}>
            <DetailRow label={t('connection.uploadSpeed')}>{calcTraffic(connection.uploadSpeed || 0)}/s</DetailRow>
            <DetailRow label={t('connection.downloadSpeed')}>{calcTraffic(connection.downloadSpeed || 0)}/s</DetailRow>
            <DetailRow label={t('connection.uploadAmount')}>{calcTraffic(connection.upload)}</DetailRow>
            <DetailRow label={t('connection.downloadAmount')}>{calcTraffic(connection.download)}</DetailRow>
          </Section>

          {/* Routing */}
          <Section title={t('connections.sections.routing')}>
            <CopyableValue
              label={t('connection.rule')}
              value={
                connection.rule
                  ? `${connection.rule}${connection.rulePayload ? `(${connection.rulePayload})` : ''}`
                  : t('connection.noRuleMatched')
              }
              prefix={[]}
            />
            <DetailRow label={t('connection.proxyChain')}>{[...connection.chains].reverse().join(' → ')}</DetailRow>
            <CopyableValue
              label={t('connection.connectionType')}
              value={[connection.metadata.type, connection.metadata.network]}
              displayName={`${connection.metadata.type}(${connection.metadata.network})`}
              prefix={['IN-TYPE', 'NETWORK']}
            />
          </Section>

          {/* Network */}
          {(connection.metadata.host ||
            connection.metadata.sniffHost ||
            connection.metadata.destinationIP ||
            connection.metadata.destinationGeoIP?.length ||
            connection.metadata.destinationIPASN ||
            connection.metadata.sourceIP ||
            connection.metadata.sourceGeoIP?.length ||
            connection.metadata.sourceIPASN ||
            connection.metadata.sourcePort ||
            connection.metadata.destinationPort ||
            connection.metadata.remoteDestination) && (
            <Section title={t('connections.sections.network')}>
              {connection.metadata.host && (
                <CopyableValue
                  label={t('connection.host')}
                  value={connection.metadata.host}
                  prefix={['DOMAIN', 'DOMAIN-SUFFIX']}
                />
              )}
              {connection.metadata.sniffHost && (
                <CopyableValue
                  label={t('connection.sniffHost')}
                  value={connection.metadata.sniffHost}
                  prefix={['DOMAIN', 'DOMAIN-SUFFIX']}
                />
              )}
              {connection.metadata.destinationIP && (
                <CopyableValue
                  label={t('connection.destinationIP')}
                  value={connection.metadata.destinationIP}
                  prefix={['IP-CIDR']}
                />
              )}
              {connection.metadata.destinationGeoIP && connection.metadata.destinationGeoIP.length > 0 && (
                <CopyableValue
                  label={t('connection.destinationGeoIP')}
                  value={connection.metadata.destinationGeoIP}
                  prefix={['GEOIP']}
                />
              )}
              {connection.metadata.destinationIPASN && (
                <CopyableValue
                  label={t('connection.destinationASN')}
                  value={connection.metadata.destinationIPASN}
                  prefix={['IP-ASN']}
                />
              )}
              {connection.metadata.sourceIP && (
                <CopyableValue
                  label={t('connection.sourceIP')}
                  value={connection.metadata.sourceIP}
                  prefix={['SRC-IP-CIDR']}
                />
              )}
              {connection.metadata.sourceGeoIP && connection.metadata.sourceGeoIP.length > 0 && (
                <CopyableValue
                  label={t('connection.sourceGeoIP')}
                  value={connection.metadata.sourceGeoIP}
                  prefix={['SRC-GEOIP']}
                />
              )}
              {connection.metadata.sourceIPASN && (
                <CopyableValue
                  label={t('connection.sourceASN')}
                  value={connection.metadata.sourceIPASN}
                  prefix={['SRC-IP-ASN']}
                />
              )}
              {connection.metadata.sourcePort && (
                <CopyableValue
                  label={t('connection.sourcePort')}
                  value={connection.metadata.sourcePort}
                  prefix={['SRC-PORT']}
                />
              )}
              {connection.metadata.destinationPort && (
                <CopyableValue
                  label={t('connection.destinationPort')}
                  value={connection.metadata.destinationPort}
                  prefix={['DST-PORT']}
                />
              )}
              {connection.metadata.remoteDestination && (
                <CopyableValue
                  label={t('connection.remoteDestination')}
                  value={connection.metadata.remoteDestination}
                  prefix={['IP-CIDR']}
                />
              )}
            </Section>
          )}

          {/* Process */}
          {connection.metadata.process && connection.metadata.type !== 'Inner' && (
            <Section title={t('connections.sections.process')}>
              <CopyableValue
                label={t('connection.processName')}
                value={[
                  connection.metadata.process,
                  ...(connection.metadata.uid ? [connection.metadata.uid.toString()] : []),
                ]}
                displayName={`${connection.metadata.process}${connection.metadata.uid ? `(${connection.metadata.uid})` : ''}`}
                prefix={['PROCESS-NAME', ...(connection.metadata.uid ? ['UID'] : [])]}
              />
              {connection.metadata.processPath && (
                <CopyableValue
                  label={t('connection.processPath')}
                  value={connection.metadata.processPath}
                  prefix={['PROCESS-PATH']}
                />
              )}
            </Section>
          )}

          {/* Inbound */}
          {(connection.metadata.inboundIP ||
            connection.metadata.inboundPort !== '0' ||
            connection.metadata.inboundName ||
            connection.metadata.inboundUser) && (
            <Section title={t('connections.sections.inbound')}>
              {connection.metadata.inboundIP && (
                <CopyableValue
                  label={t('connection.inboundIP')}
                  value={connection.metadata.inboundIP}
                  prefix={['SRC-IP-CIDR']}
                />
              )}
              {connection.metadata.inboundPort !== '0' && (
                <CopyableValue
                  label={t('connection.inboundPort')}
                  value={connection.metadata.inboundPort}
                  prefix={['SRC-PORT']}
                />
              )}
              {connection.metadata.inboundName && (
                <CopyableValue
                  label={t('connection.inboundName')}
                  value={connection.metadata.inboundName}
                  prefix={['IN-NAME']}
                />
              )}
              {connection.metadata.inboundUser && (
                <CopyableValue
                  label={t('connection.inboundUser')}
                  value={connection.metadata.inboundUser}
                  prefix={['IN-USER']}
                />
              )}
            </Section>
          )}

          {/* Other */}
          {(connection.metadata.dscp !== 0 ||
            connection.metadata.dnsMode ||
            connection.metadata.specialProxy ||
            connection.metadata.specialRules) && (
            <Section title={t('connections.sections.other')}>
              {connection.metadata.dscp !== 0 && (
                <CopyableValue label='DSCP' value={connection.metadata.dscp.toString()} prefix={['DSCP']} />
              )}
              {connection.metadata.dnsMode && (
                <DetailRow label={t('connection.dnsMode')}>{connection.metadata.dnsMode}</DetailRow>
              )}
              {connection.metadata.specialProxy && (
                <DetailRow label={t('connection.specialProxy')}>{connection.metadata.specialProxy}</DetailRow>
              )}
              {connection.metadata.specialRules && (
                <DetailRow label={t('connection.specialRule')}>{connection.metadata.specialRules}</DetailRow>
              )}
            </Section>
          )}
        </div>

        <DialogFooter>
          <DialogClose asChild>
            <Button size='sm' variant='outline'>
              {t('common.close')}
            </Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default ConnectionDetailModal;
