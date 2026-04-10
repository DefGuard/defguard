import { useState } from 'react';
import { m } from '../../paraglide/messages';
import { AppText } from '../../shared/defguard-ui/components/AppText/AppText';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { ExternalLink } from '../../shared/defguard-ui/components/ExternalLink/ExternalLink';
import { Helper } from '../../shared/defguard-ui/components/Helper/Helper';
import { Icon } from '../../shared/defguard-ui/components/Icon/Icon';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../shared/defguard-ui/types';
import { Thumbnail } from '../../shared/video-tutorials/components/widget/Thumbnail/Thumbnail';
import { VideoOverlay } from '../../shared/video-tutorials/components/widget/VideoOverlay/VideoOverlay';
import { useMigrationWizardVideoGuide } from '../../shared/video-tutorials/resolved';

export const MigrationWizardVideoGuide = () => {
  const videoGuide = useMigrationWizardVideoGuide();
  const [isVideoOpen, setIsVideoOpen] = useState(false);

  if (!videoGuide) {
    return null;
  }

  return (
    <>
      <div className="migration-wizard-support">
        <SizedBox height={ThemeSpacing.Xl5} />

        <div>
          <div className="migration-wizard-support-header">
            <Helper size={16}>{m.migration_wizard_support_video_guide_helper()}</Helper>
            <AppText font={TextStyle.TBodySm500} color={ThemeVariable.FgFaded}>
              {m.migration_wizard_support_video_guide()}
            </AppText>
          </div>
          <SizedBox height={ThemeSpacing.Md} />
          <button
            type="button"
            className="migration-wizard-video-card"
            onClick={() => setIsVideoOpen(true)}
          >
            <Thumbnail
              url={`https://img.youtube.com/vi/${videoGuide.youtubeVideoId}/hqdefault.jpg`}
              title={videoGuide.title}
            />
            <div className="migration-wizard-video-info">
              <AppText
                className="migration-wizard-video-title"
                font={TextStyle.TBodySm400}
                color={ThemeVariable.FgFaded}
              >
                {videoGuide.title}
              </AppText>
            </div>
          </button>
        </div>
        <Divider spacing={ThemeSpacing.Xl2} />
        <div>
          <div className="migration-wizard-support-header">
            <Icon icon="file" size={16} staticColor={ThemeVariable.FgMuted} />
            <AppText font={TextStyle.TBodySm500} color={ThemeVariable.FgFaded}>
              {m.migration_wizard_support_related_documentation()}
            </AppText>
          </div>
          <SizedBox height={ThemeSpacing.Md} />
          <div className="migration-wizard-doc-card">
            <ExternalLink href={videoGuide.docsUrl} target="_blank" rel="noreferrer">
              {m.migration_wizard_support_documentation_link()}
            </ExternalLink>
          </div>
        </div>
      </div>

      <VideoOverlay
        video={videoGuide}
        isOpen={isVideoOpen}
        onClose={() => setIsVideoOpen(false)}
        afterClose={() => setIsVideoOpen(false)}
      />
    </>
  );
};
