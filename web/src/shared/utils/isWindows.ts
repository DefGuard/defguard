import Bowser from 'bowser';

export const isWindows = Bowser.getParser(navigator.userAgent).isOS(
  Bowser.OS_MAP.Windows,
);
