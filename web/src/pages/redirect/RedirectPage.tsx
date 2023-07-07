import './style.scss';

import { useI18nContext } from '../../i18n/i18n-react';
import { Card } from '../../shared/components/layout/Card/Card';
import SvgIconDfgOpenidRedirect from '../../shared/components/svg/IconDfgOpenidRedirect';

// used in auth flow
export const RedirectPage = () => {
  const { LL } = useI18nContext();
  return (
    <div id="redirect-page">
      <Card shaded>
        <h2>{LL.redirectPage.title()}</h2>
        <p>{LL.redirectPage.subtitle()}</p>
        <div className="icon-container">
          <SvgIconDfgOpenidRedirect />
        </div>
      </Card>
    </div>
  );
};
