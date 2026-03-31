export interface VideoTutorial {
  youtubeVideoId: string;
  title: string;
}

// outer key = version string (e.g. "2.2"), inner key = canonicalized route key (e.g. "/users")
export type VideoTutorialsMappings = Record<string, Record<string, VideoTutorial[]>>;
