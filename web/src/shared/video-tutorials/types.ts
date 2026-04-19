export interface PlayableVideo {
  youtubeVideoId: string;
  title: string;
}

export interface VideoTutorial extends PlayableVideo {
  description: string;
  /** In-app route this video is associated with (must start with "/"). */
  appRoute: string;
  /** External documentation URL. */
  docsUrl: string;
}

export interface VideoGuidePlacement extends PlayableVideo {
  /** Documentation link title shown in the migration wizard card. */
  docsTitle: string;
  /** External documentation URL. */
  docsUrl: string;
}

export interface VideoGuidePlacementGroup {
  default?: VideoGuidePlacement;
  steps?: Record<string, VideoGuidePlacement>;
}

export interface VideoTutorialsSection {
  name: string;
  videos: VideoTutorial[];
}

export interface VideoTutorialsPlacements {
  [key: string]: VideoGuidePlacementGroup | undefined;
}

export interface VideoTutorialsVersionEntry {
  sections: VideoTutorialsSection[];
  placements?: VideoTutorialsPlacements;
}

// outer key = version string (e.g. "2.0"), value = versioned tutorial payload
export type VideoTutorialsMappings = Record<string, VideoTutorialsVersionEntry>;
