export interface VideoTutorial {
  youtubeVideoId: string;
  title: string;
  description: string;
  /** In-app route this video is associated with (must start with "/"). */
  appRoute: string;
  /** External documentation URL. */
  docsUrl: string;
}

export interface VideoTutorialsSection {
  name: string;
  videos: VideoTutorial[];
}

// outer key = version string (e.g. "2.0"), value = ordered list of sections
export type VideoTutorialsMappings = Record<string, VideoTutorialsSection[]>;
