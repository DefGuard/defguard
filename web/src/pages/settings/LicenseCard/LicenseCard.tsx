import { ContentCard } from '../../../shared/components/layout/ContentCard/ContentCard';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';

export const LicenseCard = () => {
  const setLicenseModal = useModalStore((state) => state.setLicenseModal);
  const license = useAppStore((state) => state.license);
  if (!license) return null;
  return (
    <section>
      <header>
        <h2>License & Support Information</h2>
      </header>
      <ContentCard
        className="license-card"
        header={
          <h3>{license?.enterprise ? 'Enterprice' : 'Community'} license</h3>
        }
        footer={
          <>
            <p>{`licensed to: ${license?.company}`}</p>
            <p>{`expiration date: ${license?.expiration}`}</p>
          </>
        }
      >
        <div>
          {license.enterprise ? (
            <p>This includes following modules:</p>
          ) : (
            <p>Enterprice license includes:</p>
          )}
          <ul>
            <li>YubiBridge</li>
            <li>OpenID</li>
            <li>Oauth2</li>
            <li>OpenLDAP</li>
          </ul>
        </div>
        <a onClick={() => setLicenseModal({ visible: true })}>
          read license agreement
        </a>
      </ContentCard>
    </section>
  );
};
