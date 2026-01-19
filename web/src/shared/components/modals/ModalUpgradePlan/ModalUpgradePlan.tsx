import './style.scss';
import { m } from '../../../../paraglide/messages';
import { AppText } from '../../../defguard-ui/components/AppText/AppText';
import { Button } from '../../../defguard-ui/components/Button/Button';
import { Divider } from '../../../defguard-ui/components/Divider/Divider';
import { ModalFoundation } from '../../../defguard-ui/components/ModalFoundation/ModalFoundation';
import type { ModalBase } from '../../../defguard-ui/components/ModalFoundation/types';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../../defguard-ui/types';
import { Controls } from '../../Controls/Controls';
import businessBgImage from './assets/business-bg.png?url';
import enterpriseBgImage from './assets/enterprise-bg.png?url';

interface Props extends ModalBase {
  variant: 'business' | 'enterprise';
}

export const ModalUpgradePlan = ({ variant, onClose, ...foundationProps }: Props) => {
  const title = () => {
    if (variant === 'business') return 'Business';
    return 'Enterprise';
  };

  return (
    <ModalFoundation contentClassName="modal-upgrade-plan" {...foundationProps}>
      <div className="tracks">
        <div
          className="image-track"
          style={{
            backgroundImage: `url(${variant === 'business' ? businessBgImage : enterpriseBgImage})`,
          }}
        >
          <div></div>
        </div>
        <div className="content-track">
          <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
            {`Current plan limit reached`}
          </AppText>
          <SizedBox height={ThemeSpacing.Xs} />
          <AppText font={TextStyle.TTitleH4} color={ThemeVariable.FgDefault}>
            {`Upgrade to ${title()}`}
          </AppText>
          <Divider spacing={ThemeSpacing.Xl} />
          <AppText font={TextStyle.TBodyPrimary500} color={ThemeVariable.FgFaded}>
            {`You've reached the maximum number of users allowed under your current plan. To add more users, please upgrade your subscription.`}
          </AppText>
          <SizedBox height={ThemeSpacing.Lg} />
          <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
            {`Business plan also includes the following features:`}
          </AppText>
          <ul>
            <li>Activity log streaming</li>
            <li>External OpenID</li>
            <li>Firewall</li>
            <li>Client behavior</li>
          </ul>
          <SizedBox height={ThemeSpacing.Lg} />
          <AppText font={TextStyle.TBodyXs400} color={ThemeVariable.FgMuted}>
            {`To compare all available plans and choose the one that fits your needs, click the button below.`}
          </AppText>
          <SizedBox height={ThemeSpacing.Xl2} />
          <Controls>
            <div className="right">
              <Button
                variant="secondary"
                text={m.controls_cancel()}
                onClick={() => {
                  onClose?.();
                }}
              />
              <a
                href="https://defguard.net/pricing"
                target="_blank"
                rel="noopener noreferrer"
              >
                <Button text={`Check our plans`} iconRight="open-in-new-window" />
              </a>
            </div>
          </Controls>
        </div>
      </div>
    </ModalFoundation>
  );
};
