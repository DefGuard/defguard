import './style.scss';

import { ContentCard } from '../../../shared/components/layout/ContentCard/ContentCard';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import { useI18nContext } from '../../../i18n/i18n-react';
import parse from 'html-react-parser';

export const SupportCard = () => {
  const licence = useAppStore((state) => state.license);
  if (!licence) return null;
  return (
    <ContentCard header={<h3> Support </h3>} className="support">
      {licence.enterprise ? <EnterpriceContent /> : <CommunityContent />}
    </ContentCard>
  );
};

const CommunityContent = () => {
  const { LL } = useI18nContext();
  return <div>{parse(LL.settingsPage.supportCard.body.community())}</div>;
};

const EnterpriceContent = () => {
  const { LL } = useI18nContext();
  return <>{parse(LL.settingsPage.supportCard.body.enterprise())}</>;
};
