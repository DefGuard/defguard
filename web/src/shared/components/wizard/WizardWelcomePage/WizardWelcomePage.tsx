import './style.scss';
import dayjs from 'dayjs';
import { m } from '../../../../paraglide/messages';
import { AppText } from '../../../defguard-ui/components/AppText/AppText';
import { ExternalLink } from '../../../defguard-ui/components/ExternalLink/ExternalLink';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../../defguard-ui/types';
import type { WizardWelcomePageConfig } from '../types';
import { WizardTop } from '../WizardTop/WizardTop';
import fileIcon from './assets/file_icon.png';

type Props = WizardWelcomePageConfig;

export const WizardWelcomePage = ({
  title,
  subtitle,
  content,
  media,
  docsLink = 'https://docs.defguard.net/',
  docsText = m.initial_setup_wizard_welcome_docs_description(),
  onClose,
}: Props) => {
  return (
    <div className="wizard-welcome-page">
      <WizardTop onClick={onClose} />
      <SizedBox height={ThemeSpacing.Xl4} />
      <div className="content">
        <div className="main-track">
          <div className="top-content">
            <h1>{title}</h1>
            <SizedBox height={ThemeSpacing.Lg} />
            <AppText font={TextStyle.TBodyPrimary400} color={ThemeVariable.FgFaded}>
              {subtitle}
            </AppText>
            <div className="left">{content}</div>
          </div>
          <div id="docs-card">
            <div className="image-track">
              <img src={fileIcon} alt={m.initial_setup_wizard_welcome_docs_alt()} />
            </div>
            <div className="content">
              <p>{docsText}</p>
              <div>
                <ExternalLink href={docsLink}>
                  {m.initial_setup_wizard_welcome_docs_link()}
                </ExternalLink>
              </div>
            </div>
          </div>
        </div>
        <div className="media-track">{media}</div>
      </div>
      <div className="footer">
        <p>{m.footer_copyright({ year: dayjs().year() })}</p>
        <p>
          {m.initial_setup_wizard_footer_support_text()}{' '}
          <a href="mailto:support@defguard.net" className="mail">
            support@defguard.net
          </a>
        </p>
      </div>
    </div>
  );
};
