import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { useNavigationStore } from '../../hooks/useNavigationStore';

export const ApplicationVersion = () => {
  const version = useAppStore((store) => store.appInfo?.version);
  const { LL } = useI18nContext();
  const isOpen = useNavigationStore((state) => state.isOpen);

  return (
    <div className="app-version">
      <p>
        {isOpen && <>{LL.navigation.copyright()}</>}
        <a href="https://www.teonite.com" target="_blank" rel="noreferrer">
          {!isOpen && '\u00A9'}teonite
        </a>
      </p>
      {version && (
        <p>
          <a
            rel="noreferrer"
            href={`https://github.com/DefGuard/defguard/releases/tag/v${version}`}
          >
            {isOpen
              ? LL.navigation.version.open({ version })
              : LL.navigation.version.closed({ version })}
          </a>
        </p>
      )}
    </div>
  );
};
