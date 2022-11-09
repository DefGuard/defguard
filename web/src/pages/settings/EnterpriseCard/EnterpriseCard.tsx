import { useQuery } from '@tanstack/react-query';

import { ContentCard } from '../../../shared/components/layout/ContentCard/ContentCard';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import useApi from '../../../shared/hooks/useApi';
import { QueryKeys } from '../../../shared/queries';

export const EnterpriseCard = () => {
  const setLicenseModal = useModalStore((state) => state.setLicenseModal);
  const {
    license: { getLicense },
  } = useApi();
  const { data: license } = useQuery([QueryKeys.FETCH_LICENSE], getLicense);
  return (
    <ContentCard
      title="Enterprise license"
      footer={
        <p>
          {`Licensed to: ${license?.company}`}
          <br /> {`expiration date: ${license?.expiration}`}
        </p>
      }
    >
      <p>
        If you wish to get Enterprise license for full features set,
        <br /> support, please visit{' '}
        <a href="https://defguard.net">defguard.net</a>
      </p>
      <br />
      <ul>
        <li>YubiBridge</li>
        <li>OpenID</li>
        <li>Oauth2</li>
        <li>OpenLDAP</li>
      </ul>
      <br />
      <a onClick={() => setLicenseModal({ visible: true })}>
        read license agreement
      </a>
    </ContentCard>
  );
};
