import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { isUndefined, sortBy } from 'lodash-es';
import { Fragment, useMemo } from 'react';

import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../../shared/queries';
import { ApiTokenItem } from './ApiTokenItem/ApiTokenItem';

export const ApiTokenList = () => {
  const user = useUserProfileStore((s) => s.userProfile?.user);
  const {
    user: { getApiTokensInfo: fetchApiTokens },
  } = useApi();

  const { data: apiTokens } = useQuery({
    queryFn: () => fetchApiTokens({ username: user?.username as string }),
    queryKey: [QueryKeys.FETCH_API_TOKENS_INFO, user?.username],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
    enabled: !isUndefined(user),
  });

  const sortedTokens = useMemo(() => {
    if (apiTokens) {
      return sortBy(apiTokens, (token) => token.name.toLowerCase());
    }
    return [];
  }, [apiTokens]);

  if (apiTokens?.length === 0 || !apiTokens) return null;

  return (
    <div className="api-token-list">
      {sortedTokens.map((token, index) => (
        <Fragment key={token?.id ?? index}>
          {token && <ApiTokenItem tokenData={token} />}
        </Fragment>
      ))}
    </div>
  );
};
