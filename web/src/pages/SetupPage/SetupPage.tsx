import './style.scss';
import { Controls } from '../../shared/components/Controls/Controls';
import { NavLogo } from '../../shared/components/Navigation/assets/NavLogo';
import { AppText } from '../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { ExternalLink } from '../../shared/defguard-ui/components/ExternalLink/ExternalLink';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../shared/defguard-ui/types';
import fileIcon from './assets/file_icon.png';
import worldMap from './assets/world_map.mp4';
import worldMapPoster from './assets/world_map_poster.png';

export const SetupPage = () => {
  return (
    <div id="setup-page">
      <div className="content-limiter">
        <div className="page-grid">
          <header>
            <NavLogo />
          </header>
          <div id="content-card">
            <div className="main-track">
              <h1>Welcome to Defguard initial configuration wizard.</h1>
              <SizedBox height={ThemeSpacing.Lg} />
              <AppText font={TextStyle.TBodyPrimary400} color={ThemeVariable.FgFaded}>
                {`We have detected your previous Defguard instance and here is what's going to happen`}
              </AppText>
              <SizedBox height={ThemeSpacing.Xl} />
              <Controls>
                <div className="left">
                  <Button text={`Configure Defguard`} />
                </div>
              </Controls>
              <SizedBox height={277} />
              <div id="docs-card">
                <div className="image-track">
                  <img src={fileIcon} />
                </div>
                <div className="content">
                  <p>{`Before installation, we recommend reading our documentation to understand the system architecture and core components.`}</p>
                  <div>
                    <ExternalLink href="https://docs.defguard.net/">
                      {`Read documentation`}
                    </ExternalLink>
                  </div>
                </div>
              </div>
            </div>
            <div className="image">
              <video
                height={657}
                width={443}
                autoPlay
                loop
                muted
                poster={worldMapPoster}
                preload="metadata"
              >
                <source src={worldMap} type="video/mp4" />
              </video>
            </div>
          </div>
          <footer>
            <div>
              <p>
                <span>
                  Copyright ©2023-2025{' '}
                  <a
                    href="https://defguard.net"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    Defguard
                  </a>{' '}
                  Sp. z o.o.
                </span>
              </p>
            </div>
            <div>
              <p>
                {`For help, contact our support team at `}
                <a href="mailto:support@defguard.net">{`support@defguard.net`}</a>
              </p>
            </div>
          </footer>
        </div>
      </div>
    </div>
  );
};
