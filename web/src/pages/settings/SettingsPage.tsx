import './style.scss';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/components/layout/Button/Button';
import { Card } from '../../shared/components/layout/Card/Card';
import { Helper } from '../../shared/components/layout/Helper/Helper';
import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { IconCheckmarkWhite, IconEdit } from '../../shared/components/svg';
import { useSettingsFormStore } from '../../shared/hooks/store/useSettingsFormStore';
import TeoniteLogoGif from '../../shared/images/gif/tnt-built.gif';
import { DefaultNetworkSelect } from './DefaultNetworkSelect/DefaultNetworkSelect';
import { EnterpriseCard } from './EnterpriseCard/EnterpriseCard';
import { LicenseModal } from './LicenseModal/LicenseModal';
import { SettingsForm } from './SettingsForm/SettingsForm';
import { SupportCard } from './SupportCard/SupportCard';

export const SettingsPage = () => {
  const [editMode, submitSubject, setSettingsFormState] = useSettingsFormStore(
    (state) => [state.editMode, state.submitSubject, state.setState]
  );
  return (
    <>
      <PageContainer>
        <section id="settings-page">
          <header>
            <h1>Defguard global settings</h1>
            <div className="controls">
              {editMode ? (
                <div className="right">
                  <Button
                    text="Cancel"
                    size={ButtonSize.SMALL}
                    styleVariant={ButtonStyleVariant.STANDARD}
                    onClick={() => {
                      setSettingsFormState({ editMode: false });
                    }}
                  />
                  <Button
                    text="Save Changes"
                    size={ButtonSize.SMALL}
                    styleVariant={ButtonStyleVariant.CONFIRM_SUCCESS}
                    icon={<IconCheckmarkWhite />}
                    onClick={async () => {
                      setTimeout(() => submitSubject.next(), 500);
                    }}
                  />
                </div>
              ) : (
                <Button
                  text="Edit settings"
                  icon={<IconEdit />}
                  styleVariant={ButtonStyleVariant.STANDARD}
                  onClick={() => setSettingsFormState({ editMode: true })}
                />
              )}
            </div>
          </header>
          <div className="content">
            <div className="left">
              <h2>
                Modules visibility
                <Helper>
                  <p>
                    If your not using some modules you can disable their
                    visibility.
                  </p>
                  <a href="defguard.gitbook.io" target="_blank">
                    Read more in documentation.
                  </a>
                </Helper>
              </h2>
              <Card>
                <SettingsForm />
              </Card>
              <h2>
                Default network view
                <Helper>
                  <p>Here you can change your default network view.</p>
                  <a href="defguard.gitbook.io" target="_blank">
                    Read more in documentation.
                  </a>
                </Helper>
              </h2>
              <DefaultNetworkSelect />
            </div>
            <div className="right">
              <h2>License & Support information</h2>
              <EnterpriseCard />
              <SupportCard />
              <Card className="logo-card">
                <img src={TeoniteLogoGif} alt="logo" />
              </Card>
            </div>
          </div>
        </section>
      </PageContainer>
      <LicenseModal />
    </>
  );
};
