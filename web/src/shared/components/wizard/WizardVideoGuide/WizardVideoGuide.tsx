import './style.scss';
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
  const hasVideo = Boolean(videoGuide.video);
  const hasDocs = Boolean(videoGuide.docs?.length);

  if (!hasVideo && !hasDocs) {
    return null;
  }

  return (
    <>
      <div id="wizard-video-support">
        <SizedBox height={ThemeSpacing.Xl5} />

        {hasVideo && (
          <>
            <div className="support-header">
              <Helper size={16}>{m.migration_wizard_support_video_guide_helper()}</Helper>
              <AppText font={TextStyle.TBodySm500} color={ThemeVariable.FgFaded}>
                {m.migration_wizard_support_video_guide()}
              </AppText>
            </div>
            <SizedBox height={ThemeSpacing.Md} />
            <button type="button" className="video-card" onClick={() => setIsVideoOpen(true)}>
              <Thumbnail
                url={`https://img.youtube.com/vi/${videoGuide.video!.youtubeVideoId}/hqdefault.jpg`}
                title={videoGuide.video!.title}
              />
              <div className="video-info">
                <AppText
                  className="video-title"
                  font={TextStyle.TBodySm400}
                  color={ThemeVariable.FgFaded}
                >
                  {videoGuide.video!.title}
                </AppText>
              </div>
            </button>
          </>
        )}

        {hasVideo && hasDocs && <Divider spacing={ThemeSpacing.Xl2} />}

        {hasDocs && (
          <>
            <div className="support-header">
              <Icon icon="file" size={16} staticColor={ThemeVariable.FgMuted} />
              <AppText font={TextStyle.TBodySm500} color={ThemeVariable.FgFaded}>
                {m.migration_wizard_support_related_documentation()}
              </AppText>
            </div>
            <SizedBox height={ThemeSpacing.Md} />
            <div className="doc-cards">
              {videoGuide.docs!.map((doc) => (
                <div className="doc-card" key={`${doc.docsTitle}-${doc.docsUrl}`}>
                  <ExternalLink href={doc.docsUrl} target="_blank" rel="noreferrer">
                    {doc.docsTitle}
                  </ExternalLink>
                </div>
              ))}
            </div>
          </>
        )}
      </div>
      <SizedBox height={ThemeSpacing.Xl5} />

      {hasVideo && (
        <VideoOverlay
          video={videoGuide.video!}
          isOpen={isVideoOpen}
          onClose={() => setIsVideoOpen(false)}
          afterClose={() => setIsVideoOpen(false)}
        />
      )}
    </>
  );
};
