import './style.scss';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { BuiltByCard } from './components/BuiltByCard/BuiltByCard';
import { DebugDataCard } from './components/DebugDataCard/DebugDataCard';
import { SupportCard } from './components/SupportCard/SupportCard';

export const SupportPage = () => {
  const { LL } = useI18nContext();
  return (
    <PageContainer id="support-page">
      <h1>{LL.supportPage.title()}</h1>
      <div className="content">
        <div className="left">
          <DebugDataCard />
        </div>
        <div className="right">
          <SupportCard />
          <BuiltByCard />
        </div>
      </div>
    </PageContainer>
  );
};
