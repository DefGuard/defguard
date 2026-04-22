import './styles.scss';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
  ContextualHelpKey,
  ContextualHelpSidebar,
} from '../../shared/components/ContextualHelp';
import { Page } from '../../shared/components/Page/Page';
import { SettingsCard } from '../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../shared/components/SettingsLayout/SettingsLayout';
import { externalLink } from '../../shared/constants';
import { AppText } from '../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { ButtonMenu } from '../../shared/defguard-ui/components/ButtonMenu/MenuButton';
import { ButtonsGroup } from '../../shared/defguard-ui/components/ButtonsGroup/ButtonsGroup';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { MarkedSectionHeader } from '../../shared/defguard-ui/components/MarkedSectionHeader/MarkedSectionHeader';
import docIllustration from '../../shared/defguard-ui/components/SectionSelect/assets/manual-user.png';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../shared/defguard-ui/types';
import { downloadFile } from '../../shared/utils/download';

export const SupportPage = () => {
  return (
    <Page title={m.support_page_title()}>
      <SettingsLayout
        id="support-page-content"
        suggestion={<ContextualHelpSidebar pageKey={ContextualHelpKey.Support} />}
      >
        <SettingsHeader
          icon="support"
          title={m.support_page_title()}
          subtitle={m.support_page_subtitle()}
        />
        <SettingsCard>
          <MarkedSection icon="help">
            <AppText font={TextStyle.TBodyPrimary600} color={ThemeVariable.FgDefault}>
              {m.support_page_docs_title()}
            </AppText>
            <SizedBox height={ThemeSpacing.Xl} />
            <div className="doc-highlight">
              <img src={docIllustration} alt="" className="doc-highlight-illustration" />
              <div className="doc-highlight-content">
                <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgFaded}>
                  {m.support_page_docs_desc()}
                </AppText>
                <Button
                  variant="primary"
                  text={m.support_page_docs_btn()}
                  iconRight="open-in-new-window"
                  onClick={() =>
                    window.open(
                      externalLink.defguard.docs,
                      '_blank',
                      'noopener,noreferrer',
                    )
                  }
                />
              </div>
            </div>
          </MarkedSection>
          <Divider spacing={ThemeSpacing.Xl2} />
          <MarkedSection icon="bug">
            <AppText font={TextStyle.TBodyPrimary600} color={ThemeVariable.FgDefault}>
              {m.support_page_bug_title()}
            </AppText>
            <SizedBox height={ThemeSpacing.Xl} />
            <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
              {m.support_page_bug_desc_before()}
              <AppText
                as="span"
                font={TextStyle.TBodySm500}
                color={ThemeVariable.FgMuted}
              >
                {m.support_page_bug_desc_bold()}
              </AppText>
              {m.support_page_bug_desc_after()}
            </AppText>
            <SizedBox height={ThemeSpacing.Xl} />
            <ButtonsGroup>
              <Button
                variant="secondary"
                text={m.support_page_bug_btn_report()}
                iconLeft="github"
                onClick={() =>
                  window.open(
                    externalLink.github.bugReport,
                    '_blank',
                    'noopener,noreferrer',
                  )
                }
              />
              <ButtonMenu
                variant="outlined"
                text={m.support_page_bug_btn_download()}
                iconRight="arrow-small"
                iconRightRotation="down"
                menuItems={[
                  {
                    items: [
                      {
                        text: m.support_page_bug_btn_download_support_data(),
                        onClick: async () => {
                          const res = await api.support.getSupportData();
                          const blob = new Blob([JSON.stringify(res.data, null, 2)], {
                            type: 'application/json',
                          });
                          const now = new Date().toISOString().replace(/[:.]/g, '-');
                          downloadFile(blob, `defguard-support-data-${now}`, 'json');
                        },
                      },
                    ],
                  },
                ]}
              />
            </ButtonsGroup>
          </MarkedSection>
          <Divider spacing={ThemeSpacing.Xl2} />
          <MarkedSection icon="request">
            <MarkedSectionHeader
              title={m.support_page_feature_title()}
              description={m.support_page_feature_desc()}
            />
            <ButtonsGroup>
              <Button
                variant="secondary"
                text={m.support_page_feature_btn()}
                iconLeft="github"
                onClick={() =>
                  window.open(
                    externalLink.github.featureRequest,
                    '_blank',
                    'noopener,noreferrer',
                  )
                }
              />
            </ButtonsGroup>
          </MarkedSection>
          <Divider spacing={ThemeSpacing.Xl2} />
          <MarkedSection icon="mail">
            <AppText font={TextStyle.TBodyPrimary600} color={ThemeVariable.FgDefault}>
              {m.support_page_email_title()}
            </AppText>
            <SizedBox height={ThemeSpacing.Xl} />
            <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgDefault}>
              {m.support_page_email_desc()}{' '}
              <a href="mailto:support@defguard.net">support@defguard.net</a>
            </AppText>
          </MarkedSection>
          <Divider spacing={ThemeSpacing.Xl2} />
          <MarkedSection icon="chat">
            <MarkedSectionHeader
              title={m.support_page_assistance_title()}
              description={m.support_page_assistance_desc()}
            />
            <ButtonsGroup>
              <Button
                variant="outlined"
                text={m.support_page_assistance_btn_call()}
                iconRight="calendar"
                onClick={() =>
                  window.open(
                    externalLink.defguard.scheduleCall,
                    '_blank',
                    'noopener,noreferrer',
                  )
                }
              />
            </ButtonsGroup>
          </MarkedSection>
        </SettingsCard>
      </SettingsLayout>
    </Page>
  );
};
