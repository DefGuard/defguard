import './styles.scss';
import type { ReactNode } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import { Page } from '../../shared/components/Page/Page';
import { SettingsCard } from '../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../shared/components/SettingsLayout/SettingsLayout';
import { externalLink } from '../../shared/constants';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { ButtonMenu } from '../../shared/defguard-ui/components/ButtonMenu/MenuButton';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { Icon } from '../../shared/defguard-ui/components/Icon';
import type { IconKindValue } from '../../shared/defguard-ui/components/Icon/icon-types';
import docIllustration from '../../shared/defguard-ui/components/SectionSelect/assets/manual-user.png';
import { downloadFile, downloadText } from '../../shared/utils/download';

export const SupportPage = () => {
  return (
    <Page title={m.support_page_title()}>
      <SettingsLayout id="support-page-content">
        <SettingsHeader
          icon="support"
          title={m.support_page_title()}
          subtitle={m.support_page_subtitle()}
        />
        <SettingsCard>
          <SupportSection icon="help" title={m.support_page_docs_title()}>
            <div className="doc-highlight">
              <img src={docIllustration} alt="" className="doc-highlight-illustration" />
              <div className="doc-highlight-content">
                <p>{m.support_page_docs_desc()}</p>
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
          </SupportSection>
          <Divider />
          <SupportSection
            icon="bug"
            title={m.support_page_bug_title()}
            subtitle={<BugDescription />}
          >
            <div className="support-controls">
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
                      {
                        text: m.support_page_bug_btn_download_logs(),
                        onClick: async () => {
                          const res = await api.support.getLogs();
                          const now = new Date().toISOString().replace(/[:.]/g, '-');
                          downloadText(res.data, `defguard-logs-${now}`, 'txt');
                        },
                      },
                    ],
                  },
                ]}
              />
            </div>
          </SupportSection>
          <Divider />
          <SupportSection
            icon="request"
            title={m.support_page_feature_title()}
            subtitle={m.support_page_feature_desc()}
          >
            <div className="support-controls">
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
            </div>
          </SupportSection>
          <Divider />
          <SupportSection
            icon="mail"
            title={m.support_page_email_title()}
            subtitleDark
            subtitle={
              <>
                {m.support_page_email_desc()}{' '}
                <a href="mailto:support@defguard.net">support@defguard.net</a>
              </>
            }
          />
          <Divider />
          <SupportSection
            icon="chat"
            title={m.support_page_assistance_title()}
            subtitle={m.support_page_assistance_desc()}
          >
            <div className="support-controls">
              <Button
                variant="outlined"
                text={m.support_page_assistance_btn_ticket()}
                iconRight="open-in-new-window"
                onClick={() =>
                  window.open(
                    externalLink.defguard.support,
                    '_blank',
                    'noopener,noreferrer',
                  )
                }
              />
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
            </div>
          </SupportSection>
        </SettingsCard>
      </SettingsLayout>
    </Page>
  );
};

/**
 * Renders the bug section description with the "you can optionally download"
 * portion in bold, matching the Figma design.
 */
const BugDescription = () => {
  const full = m.support_page_bug_desc();
  const boldPhrase = 'you can optionally download';
  const idx = full.indexOf(boldPhrase);
  if (idx === -1) return <>{full}</>;
  const before = full.slice(0, idx);
  const after = full.slice(idx + boldPhrase.length);
  return (
    <>
      {before}
      <strong>{boldPhrase}</strong>
      {after}
    </>
  );
};

interface SupportSectionProps {
  icon: IconKindValue;
  title: string;
  subtitle?: ReactNode;
  subtitleDark?: boolean;
  children?: ReactNode;
}

const SupportSection = ({
  icon,
  title,
  subtitle,
  subtitleDark,
  children,
}: SupportSectionProps) => {
  return (
    <div className="support-section">
      <div className="support-section-icon">
        <div className="bg" />
        <Icon icon={icon} size={20} />
      </div>
      <div className="support-section-content">
        <div className="section-header">
          <p className="section-title">{title}</p>
          {subtitle && (
            <p
              className={
                subtitleDark ? 'section-subtitle subtitle-dark' : 'section-subtitle'
              }
            >
              {subtitle}
            </p>
          )}
        </div>
        {children}
      </div>
    </div>
  );
};
