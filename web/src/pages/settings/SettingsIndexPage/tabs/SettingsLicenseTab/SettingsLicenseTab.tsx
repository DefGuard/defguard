import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { Fragment } from 'react/jsx-runtime';
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
import { getSettingsQueryOptions } from '../../../../../shared/query';
import businessImage from './assets/business.png';
import enterpriseImage from './assets/enterprise.png';
import starterImage from './assets/starter.png';
import { LicenseModal } from './modals/LicenseModal/LicenseModal';

type LicenseItemData = {
  imageSrc: string;
  title: string;
  description: string;
  badges?: BadgeProps[];
};

const licenses: Array<LicenseItemData> = [
  {
    title: 'Starter',
    imageSrc: starterImage,
    description: `Advanced protection, shared access controls, and centralized billing. Ideal for small to medium teams.`,
    badges: [{ text: 'Free', variant: BadgeVariant.Success }],
  },
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
  const licenseInfo = useApp((s) => s.appInfo.license_info);
  const { data: settings } = useQuery(getSettingsQueryOptions);

  return (
    <SettingsLayout id="settings-license-tab">
      <SettingsHeader
        icon="credit-card"
        title="License management"
        subtitle="Manage your Defguard license, view usage details and track plan limits."
      />
      {isPresent(settings) && (
        <SettingsCard>
          <div className="empty-plan">
            <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
              {`Current plan`}
            </AppText>
            <SizedBox height={ThemeSpacing.Sm} />
            <Badge variant="neutral" text={licenseInfo.tier ?? 'No plan'} />
          </div>
          <Divider spacing={ThemeSpacing.Xl} />
          <DescriptionBlock title="License key">
            <p>{`Enter your license key to unlock additional Defguard features. Your license key is sent by email after purchase or registration on the Plans page.`}</p>
          </DescriptionBlock>
          <Controls>
            <div className="left">
              <Button
                variant="primary"
                text={settings.license.length > 0 ? 'Edit license' : 'Enter license'}
                onClick={() => {
                  openModal(ModalName.License, {
                    license: settings.license,
                  });
                }}
              />
            </div>
          </Controls>
        </SettingsCard>
      )}
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
        {licenses.map((data, index) => {
          const isLast = index !== licenses.length - 1;
          return (
            <Fragment key={index}>
              <LicenseItem data={data} />
              {isLast && <Divider spacing={ThemeSpacing.Xl2} />}
            </Fragment>
          );
        })}
      </SettingsCard>
      {/* modals */}
      <LicenseModal />
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
