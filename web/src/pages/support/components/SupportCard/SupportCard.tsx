import './style.scss';

import { ReactMarkdown } from 'react-markdown/lib/react-markdown';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Divider } from '../../../../shared/defguard-ui/components/Layout/Divider/Divider';

export const SupportCard = () => {
  const { LL } = useI18nContext();
  return (
    <Card id="support-card" shaded bordered>
      <header>
        <p className="title">{LL.supportPage.supportCard.title()}</p>
      </header>
      <Divider />
      <div className="content">
        <ReactMarkdown>{LL.supportPage.supportCard.body()}</ReactMarkdown>
      </div>
    </Card>
  );
};
