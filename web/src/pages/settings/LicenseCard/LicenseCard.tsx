import { useI18nContext } from '../../../i18n/i18n-react';
import { ContentCard } from '../../../shared/components/layout/ContentCard/ContentCard';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import parse from 'html-react-parser';

export const LicenseCard = () => {
  const { LL } = useI18nContext();
  const setLicenseModal = useModalStore((state) => state.setLicenseModal);
  const license = useAppStore((state) => state.license);
  if (!license) return null;
  return (
    <section>
      <header>
        <h2>{LL.settingsPage.licenseCard.header()}</h2>
      </header>
      <ContentCard
        className="license-card"
        header={
          <h3>
            {license?.enterprise
              ? LL.settingsPage.licenseCard.licenseCardTitles.enterprise()
              : LL.settingsPage.licenseCard.licenseCardTitles.community()}{' '}
            {LL.settingsPage.licenseCard.licenseCardTitles.license()}
          </h3>
        }
        footer={
          <>
            <p>
              {LL.settingsPage.licenseCard.footer.company({
                company: license?.company,
              })}
            </p>
            <p>
              {LL.settingsPage.licenseCard.footer.expiration({
                expiration: String(license?.expiration),
              })}
            </p>
          </>
        }
      >
        <div>
          {license.enterprise
            ? parse(LL.settingsPage.licenseCard.body.enterprise())
            : parse(LL.settingsPage.licenseCard.body.community())}
          {parse(LL.settingsPage.licenseCard.body.modules())}
        </div>
        <a onClick={() => setLicenseModal({ visible: true })}>
          {LL.settingsPage.licenseCard.body.agreement()}
        </a>
      </ContentCard>
    </section>
  );
};
