export interface NavigationItem {
  title: string;
  linkPath: string;
  icon?: React.ReactNode;
  allowedToView?: string[];
  enabled: boolean | undefined;
  onClick?: () => void;
  className?: string;
}

export type NavigationTitleMapItem = {
  path: string;
  title: string;
};

export type NavigationItems = {
  middle: NavigationItem[];
  bottom: NavigationItem[];
};
