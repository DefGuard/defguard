export interface VideoSupport {
  youtubeVideoId: string;
  title: string;
}

// outer key = version string (e.g. "2.2"), inner key = canonicalized route key (e.g. "/users")
export type VideoSupportMappings = Record<string, Record<string, VideoSupport[]>>;
