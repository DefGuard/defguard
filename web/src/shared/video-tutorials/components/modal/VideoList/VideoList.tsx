import './style.scss';
import { useEffect, useMemo, useState } from 'react';
import { m } from '../../../../../paraglide/messages';
import { Fold } from '../../../../defguard-ui/components/Fold/Fold';
import { Icon } from '../../../../defguard-ui/components/Icon/Icon';
import { Search } from '../../../../defguard-ui/components/Search/Search';
import type { VideoTutorial, VideoTutorialsSection } from '../../../types';

export interface VideoListProps {
  sections: VideoTutorialsSection[];
  selectedVideo: VideoTutorial | null;
  onSelect: (video: VideoTutorial) => void;
}

export const VideoList = ({ sections, selectedVideo, onSelect }: VideoListProps) => {
  const [search, setSearch] = useState('');
  const [openSectionIndex, setOpenSectionIndex] = useState<number | null>(0);

  // When sections change (modal opens/data reloads), reset accordion to first section.
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset intentional on sections identity change
  useEffect(() => {
    setOpenSectionIndex(0);
  }, [sections]);

  const isSearching = search.trim().length > 0;

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return sections;
    return sections
      .map((section) => ({
        ...section,
        videos: section.videos.filter(
          (v) =>
            v.title.toLowerCase().includes(q) || section.name.toLowerCase().includes(q),
        ),
      }))
      .filter((s) => s.videos.length > 0);
  }, [sections, search]);

  const handleSectionToggle = (index: number, section: VideoTutorialsSection) => {
    setOpenSectionIndex((prev) => {
      if (prev === index) return null;
      // Opening a new section — select its first video
      if (section.videos.length > 0) {
        onSelect(section.videos[0]);
      }
      return index;
    });
  };

  return (
    <div className="tutorials-modal-list-panel">
      <div className="tutorials-modal-search-separator">
        <Search
          value={search}
          onChange={setSearch}
          placeholder={m.cmp_video_tutorials_modal_search_placeholder()}
        />
      </div>

      <div className="tutorials-modal-sections">
        {filtered.map((section, index) => {
          const isOpen = isSearching || openSectionIndex === index;
          return (
            <div key={section.name} className="tutorials-modal-section">
              <button
                type="button"
                className="tutorials-modal-section-header"
                onClick={() => handleSectionToggle(index, section)}
              >
                {section.name}
              </button>
              <Fold open={isOpen} contentClassName="tutorials-modal-section-videos-fold">
                <ul className="tutorials-modal-section-videos">
                  {section.videos.map((video) => {
                    const isSelected =
                      selectedVideo?.youtubeVideoId === video.youtubeVideoId;
                    return (
                      <li key={video.youtubeVideoId}>
                        <button
                          type="button"
                          className={`tutorials-modal-video-row${isSelected ? ' selected' : ''}`}
                          onClick={() => onSelect(video)}
                        >
                          <Icon icon={isSelected ? 'play-filled' : 'play'} size={16} />
                          <span>{video.title}</span>
                        </button>
                      </li>
                    );
                  })}
                </ul>
              </Fold>
            </div>
          );
        })}
      </div>
    </div>
  );
};
