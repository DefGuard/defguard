export interface HelpTutorial {
  youtubeVideoId: string;
  thumbnailUrl?: string;
  title: string;
  description?: string;
}

// outer key = version string (e.g. "2.2"), inner key = canonicalized route key (e.g. "/users")
export type HelpTutorialsMappings = Record<string, Record<string, HelpTutorial[]>>;
