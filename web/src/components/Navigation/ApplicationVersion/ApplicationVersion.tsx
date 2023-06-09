import './style.scss';

import { useI18nContext } from '../../../i18n/i18n-react';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';

export const ApplicationVersion = () => {
  const version = useAppStore((store) => store.appInfo?.version);
  const { LL } = useI18nContext();
  return (
    <div className="app-version">
      <p>
        {LL.navigation.copyright()}{' '}
        <a href="https://www.teonite.com" target="_blank" rel="noreferrer">
          teonite
        </a>
      </p>
      {version && (
        <p>
          <a
            rel="noreferrer"
            href={`https://github.com/DefGuard/defguard/releases/tag/v${version}`}
          >
            {LL.navigation.version({ version })}
          </a>
        </p>
      )}
    </div>
  );
};
