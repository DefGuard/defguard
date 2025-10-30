export const externalLink = {
  defguard: {
    download: 'https://defguard.net/download',
  },
  client: {
    desktop: {
      linux: {
        arch: 'https://aur.archlinux.org/packages/defguard-client',
      },
    },
    mobile: {
      apple: 'https://apps.apple.com/us/app/defguard-vpn-client/id6748068630',
      google: 'https://play.google.com/store/apps/details?id=net.defguard.mobile',
    },
  },
} as const;
