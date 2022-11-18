import './style.scss';

import { ContentCard } from '../../../shared/components/layout/ContentCard/ContentCard';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';

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
  return (
    <div>
      <p>For Community support Please visit:</p>
      <a href="https://github.com/Defguard/defguard">
        https://github.com/Defguard/defguard
      </a>
    </div>
  );
};

const EnterpriceContent = () => {
  return (
    <>
      <div>
        <p>For Enterprise support</p>
        <p>
          Please contact:{''}
          <a href="mailto:support@defguard.net">support@defguard.net</a>
        </p>
      </div>
      <div>
        <p>You can also visit our Community support:</p>
        <a href="https://github.com/Defguard/defguard">
          https://github.com/Defguard/defguard
        </a>
      </div>
    </>
  );
};
