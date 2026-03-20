import { m } from '../../../../../../../paraglide/messages';
import { SettingsCard } from '../../../../../../../shared/components/SettingsCard/SettingsCard';
import { externalLink } from '../../../../../../../shared/constants';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import { ExternalLink } from '../../../../../../../shared/defguard-ui/components/ExternalLink/ExternalLink';
import { SizedBox } from '../../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../../shared/defguard-ui/types';
import enterpriseImage from '../../assets/enterprise.png';

export const SettingsLicenseBusinessUpsellSection = () => {
  return (
    <SettingsCard id="license-plans">
      <header>
        <h5>{m.settings_license_expand_plan_title()}</h5>
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
            <img src={enterpriseImage} alt="" />
          </div>
          <div className="content">
            <div className="top">
              <p className="title">{m.settings_license_plan_enterprise_title()}</p>
            </div>
            <p className="description">
              {m.settings_license_plan_enterprise_description()}
            </p>
            <SizedBox height={ThemeSpacing.Md} />
            <div className="actions">
              <a
                href={externalLink.defguard.sales}
                rel="noreferrer noopener"
                target="_blank"
              >
                <Button
                  variant="outlined"
                  text={m.contact_sales()}
                />
              </a>
            </div>
          </div>
        </div>
      </div>
    </SettingsCard>
  );
};
