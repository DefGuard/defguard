import { m } from '../../../../../../../paraglide/messages';
import { SettingsCard } from '../../../../../../../shared/components/SettingsCard/SettingsCard';
import { externalLink } from '../../../../../../../shared/constants';
import { Badge } from '../../../../../../../shared/defguard-ui/components/Badge/Badge';
import { BadgeVariant } from '../../../../../../../shared/defguard-ui/components/Badge/types';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../../../../shared/defguard-ui/components/Divider/Divider';
import { ExternalLink } from '../../../../../../../shared/defguard-ui/components/ExternalLink/ExternalLink';
import { SizedBox } from '../../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../../shared/defguard-ui/types';
import businessImage from '../../assets/business.png';
import enterpriseImage from '../../assets/enterprise.png';

export const SettingsLicenseNoLicenseSection = () => {
  return (
    <SettingsCard id="license-plans">
      <header>
        <h5>{m.settings_license_choose_plan_title()}</h5>
        <ExternalLink
          href={externalLink.defguard.pricing}
          rel="noreferrer noopener"
          target="_blank"
        >
          {m.settings_license_select_plan()}
        </ExternalLink>
      </header>
      <SizedBox height={ThemeSpacing.Xl3} />
      <div className="license-item">
        <div className="track">
          <div className="image-track">
            <img src={businessImage} alt="" />
          </div>
          <div className="content">
            <div className="top">
              <p className="title">{m.settings_license_plan_business_title()}</p>
              <Badge
                text={m.settings_license_plan_business_badge()}
                variant={BadgeVariant.Plan}
              />
            </div>
            <p className="description">
              {m.settings_license_plan_business_description()}
            </p>
            <Divider spacing={ThemeSpacing.Md} />
            <p className="promotional-copy">
              {m.settings_license_plan_business_promotional_copy()}
            </p>
            <SizedBox height={ThemeSpacing.Md} />
            <div className="actions">
              <a
                href={externalLink.defguard.pricing}
                rel="noreferrer noopener"
                target="_blank"
              >
                <Button
                  variant="outlined"
                  text={m.settings_license_try_business_button()}
                  iconRight="open-in-new-window"
                />
              </a>
            </div>
          </div>
        </div>
      </div>
      <Divider spacing={ThemeSpacing.Xl2} />
      <div className="license-item">
        <div className="track">
          <div className="image-track">
            <img src={enterpriseImage} alt="" />
          </div>
          <div className="content">
            <div className="top">
              <p className="title">{m.settings_license_plan_enterprise_title()}</p>
            </div>
            <p className="description">
              {m.settings_license_plan_enterprise_description()}
            </p>
          </div>
        </div>
      </div>
    </SettingsCard>
  );
};
