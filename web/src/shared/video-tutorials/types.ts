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

export interface MigrationWizardPlacement extends PlayableVideo {
  /** External documentation URL. */
  docsUrl: string;
}

export interface VideoTutorialsSection {
  name: string;
  videos: VideoTutorial[];
}

export interface VideoTutorialsPlacements {
  migrationWizard?: MigrationWizardPlacement;
}

export interface VideoTutorialsVersionEntry {
  sections: VideoTutorialsSection[];
  placements?: VideoTutorialsPlacements;
}

// outer key = version string (e.g. "2.0"), value = versioned tutorial payload
export type VideoTutorialsMappings = Record<string, VideoTutorialsVersionEntry>;
