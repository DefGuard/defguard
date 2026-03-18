import { Fragment } from 'react';
import { m } from '../../../../../../../paraglide/messages';
import { SettingsCard } from '../../../../../../../shared/components/SettingsCard/SettingsCard';
import { externalLink } from '../../../../../../../shared/constants';
import { Badge } from '../../../../../../../shared/defguard-ui/components/Badge/Badge';
import type { BadgeProps } from '../../../../../../../shared/defguard-ui/components/Badge/types';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import { ExternalLink } from '../../../../../../../shared/defguard-ui/components/ExternalLink/ExternalLink';
import { SizedBox } from '../../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../../shared/defguard-ui/types';

export type LicensePlanCardData = {
  badges?: BadgeProps[];
  description: string;
  imageSrc: string;
  promotionalCopy?: string;
  tier: 'Business' | 'Enterprise';
  title: string;
};

type Props = {
  cards: LicensePlanCardData[];
  showTryBusinessButton?: boolean;
  variant: 'choose' | 'expand';
};

export const SettingsLicensePlansSection = ({
  cards,
  showTryBusinessButton = false,
  variant,
}: Props) => {
  return (
    <SettingsCard id="license-plans">
      <header>
        <h5>
          {variant === 'choose'
            ? m.settings_license_choose_plan_title()
            : m.settings_license_expand_plan_title()}
        </h5>
        <ExternalLink
          href={externalLink.defguard.pricing}
          rel="noreferrer noopener"
          target="_blank"
        >
          {m.settings_license_select_plan()}
        </ExternalLink>
      </header>
      <SizedBox height={ThemeSpacing.Xl3} />
      <div className="tiers">
        {cards.map((card) => (
          <Fragment key={card.tier}>
            <LicenseItem
              data={card}
              showTryBusinessButton={showTryBusinessButton && card.tier === 'Business'}
            />
          </Fragment>
        ))}
      </div>
    </SettingsCard>
  );
};

type LicenseItemProps = {
  data: LicensePlanCardData;
  showTryBusinessButton: boolean;
};

const LicenseItem = ({ data, showTryBusinessButton }: LicenseItemProps) => {
  return (
    <div className="license-item">
      <div className="track">
        <div className="image-track">
          <img src={data.imageSrc} alt="" />
        </div>
        <div className="content">
          <div className="top">
            <p className="title">{data.title}</p>
            {data.badges?.map((props) => (
              <Badge {...props} key={props.text} />
            ))}
          </div>
          <p className="description">{data.description}</p>
          {data.promotionalCopy && (
            <p className="promotional-copy">{data.promotionalCopy}</p>
          )}
          {showTryBusinessButton && (
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
          )}
        </div>
      </div>
    </div>
  );
};
