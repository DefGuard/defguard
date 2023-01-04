import './style.scss';

import { useI18nContext } from '../../../i18n/i18n-react';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';

export const ApplicationVersion = () => {
  const version = useAppStore((store) => store.version);
  const { LL } = useI18nContext();
  return (
    <div className="app-version">
      <p>
        {LL.navigation.copyright()}{' '}
        <a href="https://www.teonite.com" target="_blank" rel="noreferrer">
          teonite
        </a>
      </p>
      {version && <p>{LL.navigation.version({ version })}</p>}
    </div>
  );
};
