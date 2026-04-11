import type React from 'react';
import { type KeyboardEvent, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Kbd, KbdGroup } from '@/components/ui/kbd';
import { useAppConfig } from '@/hooks/use-app-config';
import { registerShortcut } from '@/utils/ipc';
import SettingCard from '../base/base-setting-card';
import SettingItem from '../base/base-setting-item';

const keyMap = {
  Backquote: '`',
  Backslash: '\\',
  BracketLeft: '[',
  BracketRight: ']',
  Comma: ',',
  Equal: '=',
  Minus: '-',
  Plus: 'PLUS',
  Period: '.',
  Quote: "'",
  Semicolon: ';',
  Slash: '/',
  Backspace: 'Backspace',
  CapsLock: 'Capslock',
  ContextMenu: 'Contextmenu',
  Space: 'Space',
  Tab: 'Tab',
  Convert: 'Convert',
  Delete: 'Delete',
  End: 'End',
  Help: 'Help',
  Home: 'Home',
  PageDown: 'Pagedown',
  PageUp: 'Pageup',
  Escape: 'Esc',
  PrintScreen: 'Printscreen',
  ScrollLock: 'Scrolllock',
  Pause: 'Pause',
  Insert: 'Insert',
  Suspend: 'Suspend',
};

const ShortcutConfig: React.FC = () => {
  const { t } = useTranslation();
  const { appConfig, patchAppConfig } = useAppConfig();
  const [activeAction, setActiveAction] = useState<string | null>(null);
  const {
    showWindowShortcut = '',
    triggerSysProxyShortcut = '',
    triggerTunShortcut = '',
    ruleModeShortcut = '',
    globalModeShortcut = '',
    directModeShortcut = '',
    quitWithoutCoreShortcut = '',
    restartAppShortcut = '',
  } = appConfig || {};

  return (
    <SettingCard title={t('settings.shortcuts.title')}>
      <SettingItem title={t('settings.shortcuts.toggleWindow')} divider>
        <div className='flex justify-end w-[60%]'>
          <ShortcutInput
            value={showWindowShortcut}
            patchAppConfig={patchAppConfig}
            action='showWindowShortcut'
            activeAction={activeAction}
            setActiveAction={setActiveAction}
          />
        </div>
      </SettingItem>
      <SettingItem title={t('settings.shortcuts.toggleSysProxy')} divider>
        <div className='flex justify-end w-[60%]'>
          <ShortcutInput
            value={triggerSysProxyShortcut}
            patchAppConfig={patchAppConfig}
            action='triggerSysProxyShortcut'
            activeAction={activeAction}
            setActiveAction={setActiveAction}
          />
        </div>
      </SettingItem>
      <SettingItem title={t('settings.shortcuts.toggleTun')} divider>
        <div className='flex justify-end w-[60%]'>
          <ShortcutInput
            value={triggerTunShortcut}
            patchAppConfig={patchAppConfig}
            action='triggerTunShortcut'
            activeAction={activeAction}
            setActiveAction={setActiveAction}
          />
        </div>
      </SettingItem>
      <SettingItem title={t('settings.shortcuts.switchRuleMode')} divider>
        <div className='flex justify-end w-[60%]'>
          <ShortcutInput
            value={ruleModeShortcut}
            patchAppConfig={patchAppConfig}
            action='ruleModeShortcut'
            activeAction={activeAction}
            setActiveAction={setActiveAction}
          />
        </div>
      </SettingItem>
      <SettingItem title={t('settings.shortcuts.switchGlobalMode')} divider>
        <div className='flex justify-end w-[60%]'>
          <ShortcutInput
            value={globalModeShortcut}
            patchAppConfig={patchAppConfig}
            action='globalModeShortcut'
            activeAction={activeAction}
            setActiveAction={setActiveAction}
          />
        </div>
      </SettingItem>
      <SettingItem title={t('settings.shortcuts.switchDirectMode')} divider>
        <div className='flex justify-end w-[60%]'>
          <ShortcutInput
            value={directModeShortcut}
            patchAppConfig={patchAppConfig}
            action='directModeShortcut'
            activeAction={activeAction}
            setActiveAction={setActiveAction}
          />
        </div>
      </SettingItem>
      <SettingItem title={t('settings.shortcuts.quitKeepCore')} divider>
        <div className='flex justify-end w-[60%]'>
          <ShortcutInput
            value={quitWithoutCoreShortcut}
            patchAppConfig={patchAppConfig}
            action='quitWithoutCoreShortcut'
            activeAction={activeAction}
            setActiveAction={setActiveAction}
          />
        </div>
      </SettingItem>
      <SettingItem title={t('settings.shortcuts.restartApp')}>
        <div className='flex justify-end w-[60%]'>
          <ShortcutInput
            value={restartAppShortcut}
            patchAppConfig={patchAppConfig}
            action='restartAppShortcut'
            activeAction={activeAction}
            setActiveAction={setActiveAction}
          />
        </div>
      </SettingItem>
    </SettingCard>
  );
};

const ShortcutInput: React.FC<{
  value: string;
  action: string;
  patchAppConfig: (value: Partial<AppConfig>) => Promise<void>;
  activeAction: string | null;
  setActiveAction: (action: string) => void;
}> = props => {
  const { t } = useTranslation();
  const { value, action, patchAppConfig, activeAction, setActiveAction } = props;
  const [inputValue, setInputValue] = useState(value);
  const [isFocused, setIsFocused] = useState(false);
  const displayKeys = inputValue.split('+').filter(Boolean);

  useEffect(() => {
    setInputValue(value);
  }, [value]);

  useEffect(() => {
    if (activeAction && activeAction !== action && inputValue !== value) {
      setInputValue(value);
    }
  }, [activeAction, action, inputValue, value]);

  const parseShortcut = (
    event: KeyboardEvent<HTMLElement>,
    setKey: { (value: React.SetStateAction<string>): void; (arg0: string): void },
  ): void => {
    event.preventDefault();
    let code = event.code;
    const key = event.key;
    if (code === 'Backspace') {
      setKey('');
    } else {
      let newValue = '';
      if (event.ctrlKey) {
        newValue = 'Ctrl';
      }
      if (event.shiftKey) {
        newValue = `${newValue}${newValue.length > 0 ? '+' : ''}Shift`;
      }
      if (event.metaKey) {
        newValue = `${newValue}${newValue.length > 0 ? '+' : ''}Super`;
      }
      if (event.altKey) {
        newValue = `${newValue}${newValue.length > 0 ? '+' : ''}Alt`;
      }
      if (code.startsWith('Key')) {
        code = code.substring(3);
      } else if (code.startsWith('Digit')) {
        code = code.substring(5);
      } else if (code.startsWith('Arrow')) {
        code = code.substring(5);
      } else if (key.startsWith('Arrow')) {
        code = key.substring(5);
      } else if (code.startsWith('Intl')) {
        code = code.substring(4);
      } else if (code.startsWith('Numpad')) {
        if (key.length === 1) {
          code = `Num${code.substring(6)}`;
        } else {
          code = key;
        }
      } else if (/F\d+/.test(code)) {
        // f1-f12
      } else if (keyMap[code] !== undefined) {
        code = keyMap[code];
      } else {
        code = '';
      }
      setKey(`${newValue}${newValue.length > 0 && code.length > 0 ? '+' : ''}${code}`);
    }
  };
  return (
    <>
      {inputValue !== value && (
        <Button
          className='mr-2'
          size='sm'
          onClick={async () => {
            try {
              if (await registerShortcut(value, inputValue, action)) {
                await patchAppConfig({ [action]: inputValue });
              } else {
                toast.error(t('settings.shortcuts.registerFailed'));
              }
            } catch (e) {
              toast.error(`${t('settings.shortcuts.registerFailedWithError')}${e}`);
            }
          }}
        >
          {t('settings.shortcuts.confirm')}
        </Button>
      )}
      <Button
        type='button'
        variant='outline'
        size='sm'
        className={`w-[calc(100%-72px)] h-8 justify-start gap-1 font-normal ${isFocused ? 'ring-2 ring-ring/50 ring-offset-2 ring-offset-background' : ''}`}
        onKeyDown={(e: KeyboardEvent<HTMLButtonElement>): void => {
          parseShortcut(e, setInputValue);
        }}
        onFocus={() => {
          setIsFocused(true);
          setActiveAction(action);
        }}
        onBlur={() => setIsFocused(false)}
      >
        {displayKeys.length > 0 ? (
          <KbdGroup>
            {displayKeys.map((key, index) => (
              <Kbd key={`${key}-${index}`}>{key}</Kbd>
            ))}
          </KbdGroup>
        ) : (
          <span className='text-muted-foreground text-sm'>{t('settings.shortcuts.clickToInput')}</span>
        )}
      </Button>
    </>
  );
};

export default ShortcutConfig;
