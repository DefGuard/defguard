import './style.scss';

import { ReactMarkdown } from 'react-markdown/lib/react-markdown';

import { useI18nContext } from '../../../i18n/i18n-react';
import { Card } from '../../../shared/defguard-ui/components/Layout/Card/Card';

export const SupportCard = () => {
  const { LL } = useI18nContext();
  return (
    <Card className="support">
      <h3>{LL.settingsPage.supportCard.title()}</h3>
      <ReactMarkdown>{LL.settingsPage.supportCard.body()}</ReactMarkdown>
    </Card>
  );
};
