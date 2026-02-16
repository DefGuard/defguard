import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { Fragment } from 'react/jsx-runtime';
import { LicenseTier } from '../../../../../shared/api/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../../../../shared/components/DescriptionBlock/DescriptionBlock';
import { SettingsCard } from '../../../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../../../shared/components/SettingsLayout/SettingsLayout';
import { AppText } from '../../../../../shared/defguard-ui/components/AppText/AppText';
import { Badge } from '../../../../../shared/defguard-ui/components/Badge/Badge';
import {
  type BadgeProps,
  BadgeVariant,
} from '../../../../../shared/defguard-ui/components/Badge/types';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../../shared/defguard-ui/components/Divider/Divider';
import { ExternalLink } from '../../../../../shared/defguard-ui/components/ExternalLink/ExternalLink';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import { useApp } from '../../../../../shared/hooks/useApp';
import {
  getLicenseInfoQueryOptions,
  getSettingsQueryOptions,
} from '../../../../../shared/query';
import businessImage from './assets/business.png';
import enterpriseImage from './assets/enterprise.png';
import { SettingsLicenseInfoSection } from './components/SettingsLicenseInfoSection/SettingsLicenseInfoSection';
import { SettingsLicenseModal } from './modals/SettingsLicenseModal/SettingsLicenseModal';

type LicenseItemData = {
  imageSrc: string;
  title: string;
  description: string;
  badges?: BadgeProps[];
};

const licenses: Array<LicenseItemData> = [
  {
    title: 'Business',
    imageSrc: businessImage,
    description: `Advanced protection, shared access controls, and centralized billing. Ideal for small to medium teams.`,
    badges: [{ text: 'Most popular', variant: BadgeVariant.Plan }],
  },
  {
    title: 'Enterprise',
    imageSrc: enterpriseImage,
    description: `Custom integrations, and dedicated support tailored to your organization’s security and scalability needs.`,
  },
];

export const SettingsLicenseTab = () => {
  const appLicenseInfo = useApp((s) => s.appInfo.license_info);
  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);
  const { data: settings } = useQuery(getSettingsQueryOptions);

  const licenseTier = licenseInfo?.tier ?? null;

  return (
    <SettingsLayout id="settings-license-tab">
      <SettingsHeader
        icon="credit-card"
        title="License management"
        subtitle="Manage your Defguard license, view usage details and track plan limits."
      />
      {isPresent(settings) && (
        <SettingsCard>
          {isPresent(licenseInfo) && (
            <SettingsLicenseInfoSection licenseInfo={licenseInfo} />
          )}
          {!isPresent(licenseInfo) && (
            <div className="empty-plan">
              <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
                {`Current plan`}
              </AppText>
              <SizedBox height={ThemeSpacing.Sm} />
              <Badge variant="neutral" text={appLicenseInfo.tier ?? 'No plan'} />
              <Divider spacing={ThemeSpacing.Xl} />
            </div>
          )}
          <DescriptionBlock title="License key">
            <p>{`Enter your license key to unlock additional Defguard features. Your license key is sent by email after purchase or registration on the Plans page.`}</p>
          </DescriptionBlock>
          <Controls>
            <div className="left">
              <Button
                variant="primary"
                text={
                  (settings.license?.length ?? 0) > 0 ? 'Edit license' : 'Enter license'
                }
                onClick={() => {
                  openModal(ModalName.SettingsLicense, {
                    license: settings.license,
                  });
                }}
              />
            </div>
          </Controls>
        </SettingsCard>
      )}
      {isPresent(licenseTier) && !(licenseTier === LicenseTier.Enterprise) && (
        <Fragment>
          <SizedBox height={ThemeSpacing.Xl} />
          <SettingsCard id="license-plans">
            <header>
              <h5>{`Expand your possibilities with advanced plans`}</h5>
              <ExternalLink
                href="https://defguard.net/pricing/"
                rel="noreferrer noopener"
                target="_blank"
              >
                {`Select your plan`}
              </ExternalLink>
            </header>
            <SizedBox height={ThemeSpacing.Xl3} />
            <div className="tiers">
              <LicenseItem data={licenses[1]} />
            </div>
          </SettingsCard>
        </Fragment>
      )}
      <SettingsLicenseModal />
    </SettingsLayout>
  );
};

const LicenseItem = ({ data }: { data: LicenseItemData }) => {
  return (
    <div className="license-item">
      <div className="track">
        <div className="image-track">
          <img src={data.imageSrc} />
        </div>
        <div className="content">
          <div className="top">
            <p className="title">{data.title}</p>
            {data.badges?.map((props) => (
              <Badge {...props} key={props.text} />
            ))}
          </div>
          <p className="description">{data.description}</p>
        </div>
      </div>
    </div>
  );
};
