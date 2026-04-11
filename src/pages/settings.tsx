import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import BasePage from '@/components/base/base-page';
import Actions from '@/components/settings/actions';
import AdvancedSettings from '@/components/settings/advanced-settings';
import AppearanceConfig from '@/components/settings/appearance-confis';
import GeneralConfig from '@/components/settings/general-config';
import LanguageConfig from '@/components/settings/language-config';
import ProxySwitches from '@/components/settings/proxy-switches';
import ShortcutConfig from '@/components/settings/shortcut-config';

const Settings: React.FC = () => {
  const { t } = useTranslation();
  const [showHiddenSettings, setShowHiddenSettings] = useState(false);

  return (
    <BasePage title={t('pages.settings.title')}>
      <ProxySwitches />
      <GeneralConfig showHiddenSettings={showHiddenSettings} />
      <LanguageConfig />
      <AppearanceConfig showHiddenSettings={showHiddenSettings} />
      <AdvancedSettings showHiddenSettings={showHiddenSettings} />
      <ShortcutConfig />
      <Actions showHiddenSettings={showHiddenSettings} onUnlockHiddenSettings={() => setShowHiddenSettings(true)} />
    </BasePage>
  );
};

export default Settings;
