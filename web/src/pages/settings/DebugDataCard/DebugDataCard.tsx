import { ReactMarkdown } from 'react-markdown/lib/react-markdown';

import { useI18nContext } from '../../../i18n/i18n-react';
import { ContentCard } from '../../../shared/components/layout/ContentCard/ContentCard';

export const DebugDataCard = () => {
  const { LL } = useI18nContext();
  return (
    <ContentCard
      header={<h3>{LL.settingsPage.debugDataCard.title()}</h3>}
      className="support"
    >
      <ReactMarkdown>{LL.settingsPage.debugDataCard.body()}</ReactMarkdown>
    </ContentCard>
  );
};
