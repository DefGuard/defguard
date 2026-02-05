import { useQuery } from '@tanstack/react-query';
import { type PropsWithChildren, useEffect } from 'react';
import { useApp } from '../hooks/useApp';
import { getSettingsEssentialsQueryOptions } from '../query';

export const AppSettingsEssentialsProvider = ({ children }: PropsWithChildren) => {
  const { data: settingsEssentials } = useQuery(getSettingsEssentialsQueryOptions);

  useEffect(() => {
    if (settingsEssentials) {
      useApp.setState({
        settingsEssentials,
      });
    }
  }, [settingsEssentials]);

  return children;
};
