import './style.scss';

import { useI18nContext } from '../../i18n/i18n-react';
import SvgIconDfgOpenidRedirect from '../../shared/components/svg/IconDfgOpenidRedirect';
import { Card } from '../../shared/defguard-ui/components/Layout/Card/Card';

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
