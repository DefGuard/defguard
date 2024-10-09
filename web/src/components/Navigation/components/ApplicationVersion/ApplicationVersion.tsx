import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';

type Props = {
  isOpen: boolean;
};

export const ApplicationVersion = ({ isOpen }: Props) => {
  const version = useAppStore((store) => store.appInfo?.version);
  const { LL } = useI18nContext();

  return (
    <div className="app-version">
      <p>
        <>{isOpen ? LL.navigation.copyright() : '©'}&nbsp;</>
        <a href="https://www.teonite.com" target="_blank" rel="noreferrer">
          teonite
        </a>
      </p>
      {version && (
        <p>
          <a rel="noreferrer" href={`https://github.com/DefGuard/defguard/releases/`}>
            {isOpen
              ? LL.navigation.version.open({ version })
              : LL.navigation.version.closed({
                  version: version.split('-')[0],
                })}
          </a>
        </p>
      )}
    </div>
  );
};
