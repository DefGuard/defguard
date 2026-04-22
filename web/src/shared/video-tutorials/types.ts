export interface PlayableVideo {
  youtubeVideoId: string;
  title: string;
}

export interface VideoTutorial extends PlayableVideo {
  description: string;
  /** In-app route this video is associated with (must start with "/"). */
  appRoute: string;
  /** Additional in-app routes where this tutorial should also be shown. */
  contextAppRoutes?: string[];
  /** External documentation URL. */
  docsUrl?: string;
}

export interface VideoGuideDocLink {
  /** Documentation link title shown in the wizard card. */
  docsTitle: string;
  /** External documentation URL. */
  docsUrl: string;
}

export interface VideoGuidePlacement {
  video?: PlayableVideo;
  docs?: VideoGuideDocLink[];
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
