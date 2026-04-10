import { useState } from 'react';
import { ExternalLink } from '../../shared/defguard-ui/components/ExternalLink/ExternalLink';
import { Icon } from '../../shared/defguard-ui/components/Icon/Icon';
import { ThemeVariable } from '../../shared/defguard-ui/types';
import { Thumbnail } from '../../shared/video-tutorials/components/widget/Thumbnail/Thumbnail';
import { VideoOverlay } from '../../shared/video-tutorials/components/widget/VideoOverlay/VideoOverlay';
import type { VideoTutorial } from '../../shared/video-tutorials/types';

const MIGRATION_VIDEO: VideoTutorial = {
  youtubeVideoId: 'dQw4w9WgXcQ',
  title: 'How does the certificate authority work in Defguard.',
  description: 'Temporary placeholder migration guide video.',
  appRoute: '/migration',
  docsUrl: 'https://docs.defguard.net/',
};

const VIDEO_DURATION = '00:50';
const DOCS_LABEL = 'Defguard Configuration Guide';

export const MigrationWizardVideoGuide = () => {
  const [isVideoOpen, setIsVideoOpen] = useState(false);

  return (
    <>
      <div className="migration-wizard-support">
        <div className="migration-wizard-support-section">
          <div className="migration-wizard-support-header">
            <Icon icon="help" size={20} staticColor={ThemeVariable.FgFaded} />
            <span>Video guide</span>
          </div>
          <button
            type="button"
            className="migration-wizard-video-card"
            onClick={() => setIsVideoOpen(true)}
          >
            <div className="migration-wizard-video-thumb-wrap">
              <Thumbnail
                url={`https://img.youtube.com/vi/${MIGRATION_VIDEO.youtubeVideoId}/hqdefault.jpg`}
                title={MIGRATION_VIDEO.title}
              />
              <div className="migration-wizard-video-play-badge">
                <Icon icon="tutorial" size={16} staticColor={ThemeVariable.FgAction} />
              </div>
            </div>
            <div className="migration-wizard-video-info">
              <p className="migration-wizard-video-title">{MIGRATION_VIDEO.title}</p>
              <div className="migration-wizard-video-duration">
                <Icon icon="transactions" size={16} staticColor={ThemeVariable.FgMuted} />
                <span>{VIDEO_DURATION}</span>
              </div>
            </div>
          </button>
        </div>

        <div className="migration-wizard-support-divider" />

        <div className="migration-wizard-support-section">
          <div className="migration-wizard-support-header">
            <Icon icon="file" size={20} staticColor={ThemeVariable.FgFaded} />
            <span>Related documentation</span>
          </div>
          <div className="migration-wizard-doc-card">
            <ExternalLink href={MIGRATION_VIDEO.docsUrl} target="_blank" rel="noreferrer">
              {DOCS_LABEL}
            </ExternalLink>
          </div>
        </div>
      </div>

      <VideoOverlay
        video={MIGRATION_VIDEO}
        isOpen={isVideoOpen}
        onClose={() => setIsVideoOpen(false)}
        afterClose={() => setIsVideoOpen(false)}
      />
    </>
  );
};
