import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { NotificationsForm } from './components/NotificationSettingsForm';

export const NotificationSettings = () => {
  const appInfo = useAppStore((s) => s.appInfo);

  if (!appInfo) return null;

  return <NotificationsForm />;
};
