import './VideoTutorialsModal.scss';
import { useEffect, useState } from 'react';
import { ModalFoundation } from '../defguard-ui/components/ModalFoundation/ModalFoundation';
import { isPresent } from '../defguard-ui/utils/isPresent';
import { useApp } from '../hooks/useApp';
import { ModalContent } from './components/modal/ModalContent/ModalContent';
import { useAllVideoTutorialsSections, useVideoTutorialsRouteKey } from './resolved';
import type { VideoTutorial } from './types';

export const VideoTutorialsModal = () => {
  const isOpen = useApp((s) => s.tutorialsModalOpen);
  const sections = useAllVideoTutorialsSections();
  const routeKey = useVideoTutorialsRouteKey();

  const [selectedVideo, setSelectedVideo] = useState<VideoTutorial | null>(null);

  // Auto-select first video when modal opens or sections change.
  useEffect(() => {
    if (isOpen && sections.length > 0 && sections[0].videos.length > 0) {
      setSelectedVideo(sections[0].videos[0]);
    }
  }, [isOpen, sections]);

  // Close modal on route change.
  // biome-ignore lint/correctness/useExhaustiveDependencies: routeKey is the trigger, not used in body
  useEffect(() => {
    useApp.setState({ tutorialsModalOpen: false });
  }, [routeKey]);

  const handleClose = () => useApp.setState({ tutorialsModalOpen: false });

  return (
    <ModalFoundation
      isOpen={isOpen}
      contentClassName="tutorials-modal"
      afterClose={() => setSelectedVideo(null)}
    >
      {isPresent(selectedVideo) && (
        <ModalContent
          selectedVideo={selectedVideo}
          sections={sections}
          onSelect={setSelectedVideo}
          handleClose={handleClose}
        />
      )}
    </ModalFoundation>
  );
};
