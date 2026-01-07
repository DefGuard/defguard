export enum SupportedPlatform {
  LINUX = 'LINUX',
  WINDOWS = 'WINDOWS',
  MAC = 'MAC',
}

export const checkPlatform = (): SupportedPlatform => {
  const platform = navigator.platform;
  if (platform.includes('Mac') || platform.includes('iPhone')) {
    return SupportedPlatform.MAC;
  }
  if (platform.includes('Win')) {
    return SupportedPlatform.WINDOWS;
  }
  return SupportedPlatform.LINUX;
};
