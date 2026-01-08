import axios from 'axios';
import qs from 'qs';

const baseUrl = import.meta.env.VITE_UPDATE_BASE_URL as string | undefined;

const clientDownloadFallback = 'https://defguard.net/download';

const client = axios.create({
  baseURL: baseUrl ?? 'https://pkgs.defguard.net/api',
  headers: { 'Content-Type': 'application/json' },
  paramsSerializer: {
    serialize: (params) =>
      qs.stringify(params, {
        arrayFormat: 'repeat',
      }),
  },
});

export type ClientVersionCheck = {
  windows_amd64?: string;
  deb_amd64?: string;
  deb_arm64?: string;
  deb_legacy_arm64?: string;
  deb_legacy_amd64?: string;
  rpm_amd64?: string;
  rpm_arm64?: string;
  macos_amd64?: string;
  macos_arm64?: string;
};

const updateServiceApi = {
  getClientArtifacts: () =>
    client
      .get<ClientVersionCheck>('/update/artifacts', {
        params: {
          product: 'defguard-client',
          //todo change to core
          source: 'enrollment',
        },
      })
      .then((response) => {
        const { data } = response;
        const res: ClientVersionCheck = {
          deb_amd64: data.deb_amd64 ?? clientDownloadFallback,
          deb_arm64: data.deb_arm64 ?? clientDownloadFallback,
          deb_legacy_arm64: data.deb_legacy_arm64 ?? clientDownloadFallback,
          deb_legacy_amd64: data.deb_legacy_amd64 ?? clientDownloadFallback,
          macos_amd64: data.macos_amd64 ?? clientDownloadFallback,
          macos_arm64: data.macos_arm64 ?? clientDownloadFallback,
          rpm_amd64: data.rpm_amd64 ?? clientDownloadFallback,
          rpm_arm64: data.rpm_arm64 ?? clientDownloadFallback,
          windows_amd64: data.windows_amd64 ?? clientDownloadFallback,
        };
        return res;
      })
      .catch((e) => {
        console.error(e);
        const fallback: ClientVersionCheck = {
          deb_amd64: clientDownloadFallback,
          deb_arm64: clientDownloadFallback,
          deb_legacy_arm64: clientDownloadFallback,
          deb_legacy_amd64: clientDownloadFallback,
          macos_amd64: clientDownloadFallback,
          macos_arm64: clientDownloadFallback,
          rpm_amd64: clientDownloadFallback,
          rpm_arm64: clientDownloadFallback,
          windows_amd64: clientDownloadFallback,
        };
        return fallback;
      }),
} as const;

export { updateServiceApi };
