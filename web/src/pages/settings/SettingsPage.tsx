import './style.scss';

import { useQuery } from '@tanstack/react-query';

import { Card } from '../../shared/components/layout/Card/Card';
import { CardWithHeader } from '../../shared/components/layout/CardWithHeader/CardWithHeader';
import { Helper } from '../../shared/components/layout/Helper/Helper';
import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import useApi from '../../shared/hooks/useApi';
import TeoniteLogoGif from '../../shared/images/gif/tnt-built.gif';
import { QueryKeys } from '../../shared/queries';
import { DefaultNetworkSelect } from './DefaultNetworkSelect/DefaultNetworkSelect';
import { SettingsForm } from './SettingsForm/SettingsForm';

const EnterpriseCardContent = () => {
  const {
    license: { getLicense },
  } = useApi();
  const { data: license } = useQuery([QueryKeys.FETCH_LICENSE], getLicense);
  return (
    <div className="right-card-content">
      <div className="content">
        <p>
          If you wish to get Enterprise license for full features set, support,
          please visit <a href="https://defguard.net">defguard.net</a>
        </p>
        <ul>
          <li>YubiBridge</li>
          <li>OpenID</li>
          <li>Oauth2</li>
          <li>OpenLDAP</li>
        </ul>
        <p className="link"> Read license agreement</p>
      </div>
      <div className="footer">
        <p>
          {`Licensed to: ${license?.company}`}
          <br /> {`expiration date: ${license?.expiration}`}
        </p>
      </div>
    </div>
  );
};

const SupportCard = () => {
  return (
    <div className="right-card-content">
      <div className="content">
        <p>
          For Community support Please visit:{' '}
          <a href="https://github.com/Defguard/defguard" className="link">
            github.com/Defguard/core
          </a>
        </p>
        <p>
          for Enterprise support
          <br /> Please contact:{' '}
          <span
            className="link"
            onClick={() =>
              (window.location.href = 'mailto:community@defguard.net')
            }
          >
            support@defguard.net
          </span>
        </p>
      </div>
    </div>
  );
};

const SettingsPage = () => {
  return (
    <PageContainer id="settings">
      <section id="settings-page">
        <header>
          <h1>Defguard global settings</h1>
        </header>
        <div className="content">
          <div className="left">
            <h2>
              Modules visibility
              <Helper>
                <div className="helper-content">
                  <p>
                    If your not using some modules you can disable their
                    visibility.
                  </p>
                  <a className="link-gray" href="defguard.gitbook.io">
                    Read more in documentation.
                  </a>
                </div>
              </Helper>
            </h2>
            <Card>
              <SettingsForm />
            </Card>
            <h2>
              Default network view
              <Helper>
                <div className="helper-content">
                  <p>Here you can change your default network view.</p>
                  <a className="link-gray" href="defguard.gitbook.io">
                    Read more in documentation.
                  </a>
                </div>
              </Helper>
            </h2>
            <DefaultNetworkSelect />
          </div>
          <div className="right">
            <h2>License & Support information</h2>
            <CardWithHeader header="Enterprise">
              <EnterpriseCardContent />
            </CardWithHeader>
            <CardWithHeader header="Support">
              <SupportCard />
            </CardWithHeader>

            <Card>
              <div className="logo-card-content">
                <div className="content">
                  <img src={TeoniteLogoGif} alt="logo" />
                </div>
              </div>
            </Card>
          </div>
        </div>
      </section>
    </PageContainer>
  );
};

export default SettingsPage;
