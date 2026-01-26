import './style.scss';
import { AppText } from '../../../defguard-ui/components/AppText/AppText';
import { ExternalLink } from '../../../defguard-ui/components/ExternalLink/ExternalLink';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../../defguard-ui/types';
import type { WizardWelcomePageConfig } from '../types';
import fileIcon from './assets/file_icon.png';

type Props = WizardWelcomePageConfig;

export const WizardWelcomePage = ({
  title,
  subtitle,
  content,
  media,
  docsLink = 'https://docs.defguard.net/',
  docsText = 'Before installation, we recommend reading our documentation to understand the system architecture and core components.',
}: Props) => {
  return (
    <div className="wizard-welcome-page">
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
            <img src={fileIcon} alt="Documentation" />
          </div>
          <div className="content">
            <p>{docsText}</p>
            <div>
              <ExternalLink href={docsLink}>{`Read documentation`}</ExternalLink>
            </div>
          </div>
        </div>
      </div>
      <div className="media-track">{media}</div>
    </div>
  );
};
