import { useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { AppText } from '../../../defguard-ui/components/AppText/AppText';
import { Divider } from '../../../defguard-ui/components/Divider/Divider';
import { ExternalLink } from '../../../defguard-ui/components/ExternalLink/ExternalLink';
import { Helper } from '../../../defguard-ui/components/Helper/Helper';
import { Icon } from '../../../defguard-ui/components/Icon/Icon';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../../defguard-ui/types';
import { Thumbnail } from '../../../video-tutorials/components/widget/Thumbnail/Thumbnail';
import { VideoOverlay } from '../../../video-tutorials/components/widget/VideoOverlay/VideoOverlay';
import type { VideoGuidePlacement } from '../../../video-tutorials/types';

type Props = {
  videoGuide: VideoGuidePlacement;
};

export const WizardVideoGuide = ({ videoGuide }: Props) => {
  const [isVideoOpen, setIsVideoOpen] = useState(false);

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
              {videoGuide.docsTitle}
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
