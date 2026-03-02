import { useQuery } from '@tanstack/react-query';
import { type PropsWithChildren, useEffect } from 'react';
import { getSessionInfoQueryOptions } from '../query';

export const AppSessionInfoProvider = ({ children }: PropsWithChildren) => {
  const { data } = useQuery(getSessionInfoQueryOptions);

  useEffect(() => {
    if (data) {
      console.log(data);
    }
  }, [data]);
  return children;
};
